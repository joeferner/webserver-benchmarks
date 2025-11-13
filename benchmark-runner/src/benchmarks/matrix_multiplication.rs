use std::sync::Arc;

use async_trait::async_trait;
use log::info;
use rand::{Rng, SeedableRng, rngs::StdRng};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

use crate::{Benchmark, BenchmarkError, BenchmarkOkResult, BenchmarkResult, run_requests};

type Matrix = Vec<Vec<f64>>;

const ROWS: usize = 101;
const COLUMNS: usize = 101;

struct MatrixMultiplicationBenchmark {
    matrices: Vec<Matrix>,
    expected: Vec<Matrix>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MatrixMultiplicationRequest<'a> {
    matrix1: &'a Matrix,
    matrix2: &'a Matrix,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MatrixMultiplicationResponse {
    result: Matrix,
}

#[async_trait]
impl Benchmark for MatrixMultiplicationBenchmark {
    async fn make_request(
        &self,
        client: Client,
        iteration: usize,
    ) -> Result<Response, BenchmarkError> {
        let matrix1 = &self.matrices[iteration];
        let matrix2 = &self.matrices[iteration + 1];
        let request = MatrixMultiplicationRequest { matrix1, matrix2 };
        let response = client
            .post("http://web:8000/benchmark/matrix-multiplication")
            .json(&request)
            .send()
            .await?;
        Ok(response)
    }

    async fn check_response(
        &self,
        iteration: usize,
        start: Instant,
        response: Response,
    ) -> Result<BenchmarkResult, BenchmarkError> {
        let response: MatrixMultiplicationResponse = match response.json().await {
            Ok(json) => json,
            Err(err) => {
                return Ok(BenchmarkResult::InvalidResponse(format!(
                    "Invalid JSON: {err}"
                )));
            }
        };
        let found = response.result;
        let expected = &self.expected[iteration];

        if found.len() != ROWS {
            return Ok(BenchmarkResult::InvalidResponse(format!(
                "Expected {} rows found {} rows",
                ROWS,
                found.len()
            )));
        }

        for row in 0..ROWS {
            let expected_row = &expected[row];
            let found_row = &found[row];

            if expected_row.len() != found_row.len() {
                return Ok(BenchmarkResult::InvalidResponse(format!(
                    "Expected {} columns found {} columns",
                    expected_row.len(),
                    found_row.len()
                )));
            }

            for column in 0..COLUMNS {
                let exected_value = expected_row[column];
                let found_value = found_row[column];
                if exected_value != found_value {
                    return Ok(BenchmarkResult::InvalidResponse(format!(
                        "Expected value {} found {}",
                        exected_value, found_value
                    )));
                }
            }
        }

        Ok(BenchmarkResult::Ok(BenchmarkOkResult {
            time: start.elapsed(),
            iterations: 1,
        }))
    }
}

pub async fn benchmark_matrix_multiplication(
    iterations: usize,
) -> Result<BenchmarkResult, BenchmarkError> {
    info!("benchmark matrix multiplication {iterations} iterations");

    let mut matrices: Vec<Matrix> = vec![];
    for i in 0..(iterations + 1) {
        matrices.push(generate_matrix(i as u64, ROWS, COLUMNS));
    }

    let mut expected: Vec<Matrix> = vec![];
    for i in 0..iterations {
        let matrix1 = &matrices[i];
        let matrix2 = &matrices[i + 1];
        expected.push(matrix_multiply(matrix1, matrix2));
    }

    let benchmark = MatrixMultiplicationBenchmark { matrices, expected };

    run_requests(iterations, Arc::new(benchmark)).await
}

fn new_matrix(rows: usize, columns: usize) -> Matrix {
    vec![vec![0f64; columns]; rows]
}

fn matrix_multiply(matrix1: &Matrix, matrix2: &Matrix) -> Matrix {
    let m = matrix1.len();
    let n = matrix1[0].len();
    let p = matrix2[0].len();

    let mut b2 = new_matrix(n, p);
    for (i, row) in matrix2.iter().enumerate() {
        for (j, x) in row.iter().enumerate() {
            b2[j][i] = *x;
        }
    }

    let mut c = new_matrix(m, p);
    for (ci, a_row) in c.iter_mut().zip(matrix1.iter()) {
        for (cij, b2j) in ci.iter_mut().zip(&b2) {
            *cij = a_row.iter().zip(b2j).fold(0f64, |acc, (&x, y)| acc + x * y);
        }
    }

    c
}

fn generate_matrix(seed: u64, rows: usize, columns: usize) -> Matrix {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut matrix = Vec::with_capacity(rows);

    for _i in 0..rows {
        let mut row = Vec::with_capacity(columns);
        for _j in 0..columns {
            let v: f64 = rng.random();
            row.push(v);
        }
        matrix.push(row);
    }
    matrix
}
