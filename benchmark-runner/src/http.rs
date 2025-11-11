use reqwest::Client;
use std::time::Duration;
use thiserror::Error;
use tokio::time::{Instant, sleep};

#[derive(Error, Debug)]
pub enum HttpError {
    #[error("Reqwest: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Timeout")]
    Timeout,
}

pub async fn http_wait_for_url(
    url: &str,
    check_interval: Duration,
    max_check_time: Duration,
) -> Result<(), HttpError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(5))
        .build()?;
    let start = Instant::now();

    while start.elapsed() < max_check_time {
        let is_up = client
            .get(url)
            .send()
            .await
            .and_then(|resp| resp.error_for_status())
            .is_ok();

        if is_up {
            return Ok(());
        }

        sleep(check_interval).await;
    }

    Err(HttpError::Timeout)
}
