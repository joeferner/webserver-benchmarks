use std::{
    process::Child,
    sync::{Arc, Mutex},
};

use log::info;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProcessManagerError {
    #[error("Lock")]
    Lock,
    #[error("Set Ctrl+C Handler: {0}")]
    SetHandler(#[from] ctrlc::Error),
}

pub struct ProcessManager {
    processes: Arc<Mutex<Vec<Option<Child>>>>,
}

impl ProcessManager {
    pub fn new() -> Result<Self, ProcessManagerError> {
        let processes = Arc::new(Mutex::new(vec![]));

        {
            let my_processes = processes.clone();
            ctrlc::set_handler(move || {
                stop_children(my_processes.clone()).unwrap();
                std::process::exit(0);
            })?;
        }

        Ok(Self {
            processes: Default::default(),
        })
    }

    pub fn push(&self, child: Child) -> Result<(), ProcessManagerError> {
        let mut processes = self
            .processes
            .lock()
            .map_err(|_| ProcessManagerError::Lock)?;
        processes.push(Some(child));
        Ok(())
    }
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        stop_children(self.processes.clone()).unwrap();
    }
}

fn stop_children(children: Arc<Mutex<Vec<Option<Child>>>>) -> Result<(), ProcessManagerError> {
    let mut children = children.lock().map_err(|_| ProcessManagerError::Lock)?;
    while let Some(mut child) = children.pop() {
        if let Some(mut child) = child.take() {
            info!("killing child: {}", child.id());
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    Ok(())
}
