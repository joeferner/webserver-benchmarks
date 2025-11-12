use std::{
    collections::HashMap,
    fs::File,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use env_logger::{self, Env};
use futures::future::join_all;
use log::info;
use reqwest::{Client, Response};
use serde::{Serialize, Serializer};
use thiserror::Error;
use tokio::time::Instant;

use crate::{
    benchmarks::{download_binary::benchmark_download_binary, plaintext::benchmark_plaintext},
    docker::{DockerError, run_webserver, stop_webserver},
    http::{HttpError, http_wait_for_url},
    process_manager::{ProcessManager, ProcessManagerError},
};

mod benchmarks;
mod docker;
mod http;
mod process_manager;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    info!("begin benchmarks");
    run_benchmarks().await.unwrap();
    info!("complete");
}

#[derive(Error, Debug)]
enum BenchmarkError {
    #[error("Process Manager: {0}")]
    ProcessManager(#[from] ProcessManagerError),

    #[error("IO: {0}")]
    Io(#[from] std::io::Error),

    #[error("Docker: {0}")]
    Docker(#[from] DockerError),

    #[error("HTTP: {0}")]
    Http(#[from] HttpError),

    #[error("HTTP Reqwest: {0}")]
    HttpReqwest(#[from] reqwest::Error),

    #[error("JSON Serde: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
enum BenchmarkResult {
    Ok(BenchmarkOkResult),
    InvalidStatusCode(u16),
    InvalidResponse(String),
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BenchmarkOkResult {
    #[serde(rename = "time_ms", serialize_with = "duration_as_millis")]
    time: Duration,
    iterations: usize,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BenchmarkResults {
    plaintext: BenchmarkResult,
    download_binary: BenchmarkResult,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase", untagged)]
enum BenchmarkJsonResult {
    Success(BenchmarkResults),
    Error(BenchmarkJsonError),
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BenchmarkJsonError {
    error: String,
}

fn duration_as_millis<S>(d: &Duration, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_u128(d.as_millis())
}

async fn run_benchmarks() -> Result<(), BenchmarkError> {
    let pm = ProcessManager::new()?;

    let mut all_results: HashMap<String, BenchmarkJsonResult> = HashMap::new();

    for name in ["rust-axum", "python-fastapi", "nodejs-express"] {
        let results = run_benchmark(&pm, name).await;
        match results {
            Ok(results) => {
                all_results.insert(name.to_string(), BenchmarkJsonResult::Success(results));
            }
            Err(err) => {
                all_results.insert(
                    name.to_string(),
                    BenchmarkJsonResult::Error(BenchmarkJsonError {
                        error: format!("{err}"),
                    }),
                );
            }
        }
    }

    let file = File::create("results.json")?;
    serde_json::to_writer_pretty(file, &all_results)?;

    Ok(())
}

async fn run_benchmark(
    pm: &ProcessManager,
    name: &str,
) -> Result<BenchmarkResults, BenchmarkError> {
    let child = Arc::new(Mutex::new(run_webserver(name)?));
    pm.push(child.clone())?;

    http_wait_for_url(
        "http://web:8000/benchmark/health",
        Duration::from_millis(500),
        Duration::from_secs(10),
    )
    .await?;

    let plaintext = benchmark_plaintext(10000).await?;
    let download_binary = benchmark_download_binary(1000).await?;

    stop_webserver(name)?;
    pm.kill(child)?;
    Ok(BenchmarkResults {
        plaintext,
        download_binary,
    })
}

#[async_trait]
trait Benchmark: Send + Sync {
    async fn make_request(&self, client: Client) -> Result<Response, BenchmarkError>;
    async fn check_response(
        &self,
        initial_check: bool,
        start: Instant,
        response: Response,
    ) -> Result<BenchmarkResult, BenchmarkError>;
}

async fn run_requests(
    iterations: usize,
    benchmark: Arc<dyn Benchmark>,
) -> Result<BenchmarkResult, BenchmarkError> {
    let client = Client::new();

    let response = benchmark.make_request(client.clone()).await?;
    match benchmark
        .check_response(true, Instant::now(), response)
        .await?
    {
        BenchmarkResult::Ok(_) => {}
        other => return Ok(other),
    }

    let start = Instant::now();

    let futures = (0..iterations).map(|_| {
        let client = client.clone();
        let benchmark = benchmark.clone();
        tokio::spawn(async move {
            let start = Instant::now();
            let response = benchmark.make_request(client).await?;
            let result: Result<BenchmarkResult, BenchmarkError> =
                benchmark.check_response(false, start, response).await;
            result
        })
    });

    let results = join_all(futures).await;
    let time = start.elapsed();

    for result in results {
        match result {
            Ok(result) => match result {
                Ok(result) => match result {
                    BenchmarkResult::Ok(_) => {}
                    other => return Ok(other),
                },
                Err(err) => {
                    return Ok(BenchmarkResult::InvalidResponse(format!(
                        "one or more requests failed: {err}"
                    )));
                }
            },
            Err(err) => {
                return Ok(BenchmarkResult::InvalidResponse(format!(
                    "one or more requests failed: {err}"
                )));
            }
        }
    }
    Ok(BenchmarkResult::Ok(BenchmarkOkResult { time, iterations }))
}
