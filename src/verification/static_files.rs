use crate::prelude::*;
use crate::verification::utils;
use reqwest::{Client, StatusCode};

pub async fn verify(
    client: &Client,
    base_url: &str,
    expected_size: usize,
    path: &str,
) -> Result<()> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let url = format!("{}{}", base_url, path);
    let resp = client
        .get(&url)
        .header("x-request-id", &request_id)
        .send()
        .await?;

    utils::verify_status(&resp, StatusCode::OK)?;

    let bytes = resp.bytes().await?;
    if bytes.len() != expected_size {
        return Err(Error::VerificationFailed(format!(
            "Expected {} bytes, got {}",
            expected_size,
            bytes.len()
        )));
    }
    Ok(())
}
