use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use env_logger::{self, Env};
use log::info;
use thiserror::Error;

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
}

async fn run_benchmarks() -> Result<(), BenchmarkError> {
    let pm = ProcessManager::new()?;

    run_benchmark(&pm, "fastapi").await?;

    Ok(())
}

async fn run_benchmark(pm: &ProcessManager, name: &str) -> Result<(), BenchmarkError> {
    let child = Arc::new(Mutex::new(run_webserver(name)?));
    pm.push(child.clone())?;

    http_wait_for_url(
        "http://web:8000/benchmark/health",
        Duration::from_millis(500),
        Duration::from_secs(10),
    )
    .await?;

    // do benchmark

    pm.kill(child)?;
    Ok(())
}
