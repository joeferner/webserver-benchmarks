use std::sync::Arc;

use async_trait::async_trait;
use log::info;
use reqwest::{Client, Response, StatusCode};
use tokio::time::Instant;

use crate::{Benchmark, BenchmarkError, BenchmarkOkResult, BenchmarkResult, run_requests};

struct PlaintextBenchmark {}

#[async_trait]
impl Benchmark for PlaintextBenchmark {
    async fn make_request(
        &self,
        client: Client,
        _iteration: usize,
    ) -> Result<Response, BenchmarkError> {
        let response = client
            .get("http://web:8000/benchmark/plain-text")
            .send()
            .await?;
        Ok(response)
    }

    async fn check_response(
        &self,
        _iteration: usize,
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
}

pub async fn benchmark_plaintext(iterations: usize) -> Result<BenchmarkResult, BenchmarkError> {
    info!("benchmark plaintext {iterations} iterations");
    let benchmark = PlaintextBenchmark {};
    run_requests(iterations, Arc::new(benchmark)).await
}
