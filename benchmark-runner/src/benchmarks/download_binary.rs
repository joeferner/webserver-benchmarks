use std::{
    fs::{self},
    sync::Arc,
};

use async_trait::async_trait;
use bytes::Bytes;
use log::info;
use reqwest::{Client, Response, StatusCode};
use tokio::time::Instant;

use crate::{Benchmark, BenchmarkError, BenchmarkOkResult, BenchmarkResult, run_requests};

struct DownloadBinaryBenchmark {
    binary_data: Bytes,
}

#[async_trait]
impl Benchmark for DownloadBinaryBenchmark {
    async fn make_request(&self, client: Client) -> Result<Response, BenchmarkError> {
        let response = client
            .get("http://web:8000/benchmark/download-binary")
            .send()
            .await?;
        Ok(response)
    }

    async fn check_response(
        &self,
        _initial_check: bool,
        start: Instant,
        response: Response,
    ) -> Result<BenchmarkResult, BenchmarkError> {
        if response.status() != StatusCode::OK {
            return Ok(BenchmarkResult::InvalidStatusCode(
                response.status().as_u16(),
            ));
        }

        let bytes = response.bytes().await?;
        if bytes != self.binary_data {
            if bytes.len() != self.binary_data.len() {
                return Ok(BenchmarkResult::InvalidResponse(format!(
                    "Expected bytes length {} found bytes len {}",
                    self.binary_data.len(),
                    bytes.len()
                )));
            }
            return Ok(BenchmarkResult::InvalidResponse(
                "Bytes data mismatch".to_string(),
            ));
        }

        Ok(BenchmarkResult::Ok(BenchmarkOkResult {
            time: start.elapsed(),
            iterations: 1,
        }))
    }
}

pub async fn benchmark_download_binary(
    iterations: usize,
) -> Result<BenchmarkResult, BenchmarkError> {
    info!("benchmark download binary {iterations} iterations");

    let binary_data = Bytes::from(fs::read("/assets/download-binary.png")?);
    let benchmark = DownloadBinaryBenchmark { binary_data };

    run_requests(iterations, Arc::new(benchmark)).await
}
