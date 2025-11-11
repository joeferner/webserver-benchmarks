use std::io::Error;
use std::process::{Child, Command};

use log::info;

pub fn run_webserver(name: &str) -> Result<Child, Error> {
    let mut cmd = Command::new("docker");
    let args = ["compose", "up", "--build", "--remove-orphans"];

    info!("spawning webserver: {name}");
    cmd.args(args)
        .current_dir(format!("/webservers/{name}"))
        .spawn()
}

pub fn stop_webserver(name: &str) -> Result<(), Error> {
    let mut cmd = Command::new("docker");
    let args = ["compose", "stop"];

    info!("stopping webserver: {name}");
    cmd.args(args)
        .current_dir(format!("/webservers/{name}"))
        .output()?;
    Ok(())
}
