use std::{
    process::Child,
    sync::{Arc, Mutex},
};

use log::{debug, error, info};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProcessManagerError {
    #[error("Lock")]
    Lock,
    #[error("Set Ctrl+C Handler: {0}")]
    SetHandler(#[from] ctrlc::Error),
    #[error("Child not found: {0}")]
    ChildNotFound(u32),
}

pub struct ProcessManager {
    processes: Arc<Mutex<Vec<Option<Arc<Mutex<Child>>>>>>,
}

impl ProcessManager {
    pub fn new() -> Result<Self, ProcessManagerError> {
        let processes = Arc::new(Mutex::new(vec![]));

        {
            let my_processes = processes.clone();
            ctrlc::set_handler(move || {
                kill_processes(my_processes.clone()).unwrap();
                std::process::exit(0);
            })?;
        }

        Ok(Self {
            processes: Default::default(),
        })
    }

    pub fn push(&self, child: Arc<Mutex<Child>>) -> Result<(), ProcessManagerError> {
        let mut processes = self.processes.lock().map_err(|err| {
            error!("failed to lock process: {err}");
            ProcessManagerError::Lock
        })?;
        processes.push(Some(child));
        Ok(())
    }

    pub fn kill(&self, child: Arc<Mutex<Child>>) -> Result<(), ProcessManagerError> {
        let pid = child
            .lock()
            .map_err(|err| {
                error!("failed to lock process: {err}");
                ProcessManagerError::Lock
            })?
            .id();
        let mut children = self.processes.lock().map_err(|err| {
            error!("failed to lock process: {err}");
            ProcessManagerError::Lock
        })?;
        let pos = {
            let mut lock_error = false;
            let pos = children.iter().position(|c| {
                if let Some(c) = c {
                    match c.lock() {
                        Ok(c) => c.id() == pid,
                        Err(err) => {
                            error!("failed to lock process: {err}");
                            lock_error = true;
                            false
                        }
                    }
                } else {
                    false
                }
            });
            if lock_error {
                return Err(ProcessManagerError::Lock);
            }
            pos
        };
        if let Some(pos) = pos {
            if let Some(child) = children.remove(pos) {
                kill_process(child)?;
            }
        } else {
            return Err(ProcessManagerError::ChildNotFound(pid));
        }
        Ok(())
    }
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        kill_processes(self.processes.clone()).unwrap();
    }
}

fn kill_processes(
    processes: Arc<Mutex<Vec<Option<Arc<Mutex<Child>>>>>>,
) -> Result<(), ProcessManagerError> {
    let mut processes = processes.lock().map_err(|err| {
        error!("failed to lock process: {err}");
        ProcessManagerError::Lock
    })?;
    while let Some(mut process) = processes.pop() {
        if let Some(process) = process.take() {
            kill_process(process)?;
        }
    }
    Ok(())
}

fn kill_process(process: Arc<Mutex<Child>>) -> Result<(), ProcessManagerError> {
    let mut process = process.lock().map_err(|err| {
        error!("failed to lock process: {err}");
        ProcessManagerError::Lock
    })?;
    match process.try_wait() {
        Ok(Some(status)) => {
            debug!("process already exited (pid: {}): {status}", process.id());
        }
        Ok(None) => {
            info!("killing process: {}", process.id());
            let _ = process.kill();
            let _ = process.wait();
        }
        Err(err) => {
            error!(
                "failed to wait for child process: (pid: {}): {err}",
                process.id()
            )
        }
    }
    Ok(())
}
