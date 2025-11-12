use std::cmp::Reverse;
use std::process::{Child, Command};

use log::{debug, info};
use serde::Deserialize;
use serde_json::from_str;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DockerError {
    #[error("IO: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Other: {0}")]
    Other(String),
}

#[derive(Deserialize, Debug)]
struct Container {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "Image")]
    image_name: String,
    #[serde(rename = "CreatedAt")]
    created_at: String,
}

#[derive(Deserialize, Debug, Clone)]
struct Inspect {
    #[serde(rename = "Mounts")]
    mounts: Vec<InspectMount>,
}

#[derive(Deserialize, Debug, Clone)]
struct InspectMount {
    #[serde(rename = "Source")]
    source: String,
    #[serde(rename = "Destination")]
    destination: String,
}

pub fn run_webserver(name: &str) -> Result<Child, DockerError> {
    let mut cmd = Command::new("docker");
    let args = ["compose", "up", "--build", "--remove-orphans"];
    let assets_dir = get_assets_dir()?;

    info!("spawning webserver: {name} (assets_dir: {assets_dir})");
    let child = cmd
        .args(args)
        .env("ASSETS_PATH", assets_dir)
        .current_dir(format!("/webservers/{name}"))
        .spawn()?;
    Ok(child)
}

pub fn stop_webserver(name: &str) -> Result<(), DockerError> {
    let mut cmd = Command::new("docker");
    let args = ["compose", "stop"];

    info!("stopping webserver: {name}");
    cmd.args(args)
        .current_dir(format!("/webservers/{name}"))
        .output()?;
    Ok(())
}

fn get_assets_dir() -> Result<String, DockerError> {
    debug!("getting assets dir");

    let image_name = "benchmark-runner-benchmark-runner";
    let mut containers = docker_ps()?;
    containers.sort_by_key(|c| Reverse(c.created_at.clone()));

    for container in containers {
        if container.image_name != image_name {
            continue;
        }

        debug!("inspecting container for mounts: {}", container.id);
        let inspect = docker_inspect(&container.id)?;
        for mount in inspect.mounts {
            if mount.destination == "/assets" {
                return Ok(mount.source);
            }
        }
    }

    Err(DockerError::Other(format!(
        "Could not find docker container with image name: {image_name}"
    )))
}

fn docker_inspect(id: &str) -> Result<Inspect, DockerError> {
    let mut cmd = Command::new("docker");
    let args = ["inspect", "--format", "json", id];

    let output = cmd.args(args).output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let inspect = from_str::<Vec<Inspect>>(&stdout)?;
    Ok(inspect[0].clone())
}

fn docker_ps() -> Result<Vec<Container>, DockerError> {
    let mut cmd = Command::new("docker");
    let args = ["ps", "-a", "--format", "json"];

    let output = cmd.args(args).output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let containers: Result<Vec<Container>, serde_json::Error> = stdout
        .lines()
        .filter(|line| !line.trim().is_empty()) // skip blank lines
        .map(from_str::<Container>)
        .collect();
    Ok(containers?)
}
