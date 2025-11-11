use std::{
    collections::HashMap,
    fs::File,
    sync::{Arc, Mutex},
    time::Duration,
};

use env_logger::{self, Env};
use futures::future::join_all;
use log::info;
use reqwest::{Client, Response, StatusCode};
use serde::{Serialize, Serializer};
use thiserror::Error;
use tokio::time::Instant;

use crate::{
    docker::{run_webserver, stop_webserver},
    http::{HttpError, http_wait_for_url},
    process_manager::{ProcessManager, ProcessManagerError},
};

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
struct BenchmarkOkResult {
    #[serde(rename = "time_ms", serialize_with = "duration_as_millis")]
    time: Duration,
    iterations: usize,
}

#[derive(Serialize, Debug)]
struct BenchmarkResults {
    plaintext: BenchmarkResult,
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
enum BenchmarkJsonResult {
    Success(BenchmarkResults),
    Error(BenchmarkJsonError),
}

#[derive(Serialize, Debug)]
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

    for name in ["fastapi", "nodejs-express"] {
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

    stop_webserver(name)?;
    pm.kill(child)?;
    Ok(BenchmarkResults { plaintext })
}

async fn benchmark_plaintext(iterations: usize) -> Result<BenchmarkResult, BenchmarkError> {
    async fn make_request(client: Client) -> Result<Response, BenchmarkError> {
        let response = client
            .get("http://web:8000/benchmark/plain-text")
            .send()
            .await?;
        Ok(response)
    }

    async fn check_response(
        start: Instant,
        response: Response,
    ) -> Result<BenchmarkResult, BenchmarkError> {
        if response.status() != StatusCode::OK {
            return Ok(BenchmarkResult::InvalidStatusCode(
                response.status().as_u16(),
            ));
        }
        let text = response.text().await?;
        if text != "Hello, World!" {
            return Ok(BenchmarkResult::InvalidResponse(format!(
                "Expected \"Hello, World!\" found \"{text}\""
            )));
        }

        Ok(BenchmarkResult::Ok(BenchmarkOkResult {
            time: start.elapsed(),
            iterations: 1,
        }))
    }

    run_requests(iterations, make_request, check_response).await
}

async fn run_requests<FMakeRequest, RetMakeRequest, FCheckResponse, RetCheckResponse>(
    iterations: usize,
    make_request: FMakeRequest,
    check_response: FCheckResponse,
) -> Result<BenchmarkResult, BenchmarkError>
where
    FMakeRequest: Fn(Client) -> RetMakeRequest + Send + Sync + Clone + 'static,
    RetMakeRequest: Future<Output = Result<Response, BenchmarkError>> + Send,
    FCheckResponse: Fn(Instant, Response) -> RetCheckResponse + Send + Sync + Clone + 'static,
    RetCheckResponse: Future<Output = Result<BenchmarkResult, BenchmarkError>> + Send,
{
    let client = Client::new();

    let response = make_request(client.clone()).await?;
    match check_response(Instant::now(), response).await? {
        BenchmarkResult::Ok(_) => {}
        other => return Ok(other),
    }

    let start = Instant::now();

    let futures = (0..iterations).map(|_| {
        let client = client.clone();
        let make_request = make_request.clone();
        let check_response = check_response.clone();
        tokio::spawn(async move {
            let start = Instant::now();
            let response = make_request(client).await?;
            let result: Result<BenchmarkResult, BenchmarkError> =
                check_response(start, response).await;
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
