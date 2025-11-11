use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use env_logger::{self, Env};
use futures::future::join_all;
use log::info;
use reqwest::{Client, Response, StatusCode};
use thiserror::Error;
use tokio::time::Instant;

use crate::{
    docker::run_webserver,
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
}

#[derive(Debug)]
enum BenchmarkResult {
    Ok(Duration),
    InvalidStatusCode(StatusCode),
    InvalidResponse(String),
}

#[derive(Debug)]
struct BenchmarkResults {
    plaintext: BenchmarkResult,
}

async fn run_benchmarks() -> Result<(), BenchmarkError> {
    let pm = ProcessManager::new()?;

    let fastapi = run_benchmark(&pm, "fastapi").await?;
    println!();
    println!();
    println!("Results");
    println!("  fastapi: {fastapi:?}");
    println!();
    println!();

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

    let plaintext = benchmark_plaintext(1000).await?;

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
            return Ok(BenchmarkResult::InvalidStatusCode(response.status()));
        }
        let text = response.text().await?;
        if text != "Hello, World!" {
            return Ok(BenchmarkResult::InvalidResponse(format!(
                "Expected \"Hello, World!\" found \"{text}\""
            )));
        }

        Ok(BenchmarkResult::Ok(start.elapsed()))
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

    validate_results(time, results)
}

fn validate_results(
    time: Duration,
    results: Vec<Result<Result<BenchmarkResult, BenchmarkError>, tokio::task::JoinError>>,
) -> Result<BenchmarkResult, BenchmarkError> {
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
    Ok(BenchmarkResult::Ok(time))
}
