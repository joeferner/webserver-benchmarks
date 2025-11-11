use env_logger::{self, Env};
use log::info;
use thiserror::Error;

use crate::{
    docker::run_webserver,
    process_manager::{ProcessManager, ProcessManagerError},
};

mod docker;
mod process_manager;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    info!("begin benchmarks");
    run_benchmarks().unwrap();
    info!("complete");
}

#[derive(Error, Debug)]
enum BenchmarkError {
    #[error("Process Manager: {0}")]
    ProcessManager(#[from] ProcessManagerError),
    #[error("IO: {0}")]
    Io(#[from] std::io::Error),
}

fn run_benchmarks() -> Result<(), BenchmarkError> {
    let pm = ProcessManager::new()?;

    let child = run_webserver("fastapi")?;
    pm.push(child)?;

    Ok(())
}
