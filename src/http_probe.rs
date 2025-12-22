use crate::prelude::*;
use std::time::Duration;

#[allow(clippy::collapsible_if)]
pub async fn wait_server_ready(host: &str, timeout: Duration) -> Result<()> {
    let client = reqwest::Client::new();
    let start = tokio::time::Instant::now();
    loop {
        if let Ok(resp) = client.get(format!("http://{}/health", host)).send().await
            && resp.status().is_success()
            && resp.text().await.unwrap_or_default() == "OK"
        {
            return Ok(());
        }
        if start.elapsed() > timeout {
            return Err(Error::ServerStartTimeoutError);
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
