use crate::{benchmark::BenchmarkTests, prelude::*};
use std::{collections::VecDeque, time::Duration};

pub struct ServerInfo {
    pub version: String,
    pub supported_tests: Vec<BenchmarkTests>,
}

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

pub async fn get_server_version(host: &str) -> Result<ServerInfo> {
    let client = reqwest::Client::new();
    let resp = client.get(format!("http://{}/info", host)).send().await?;
    let result = resp.text().await?;
    let mut parts = result.split(',').collect::<VecDeque<&str>>();
    let mut version = "unknown".to_string();
    let mut supported_tests = Vec::new();
    if let Some(v) = parts.pop_front() {
        version = v.to_string();
    }
    for test in parts {
        supported_tests.push(test.try_into().map_err(Error::ServerInfoParseError)?);
    }
    Ok(ServerInfo {
        version,
        supported_tests,
    })
}
