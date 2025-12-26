use crate::prelude::*;
use crate::verification::utils;
use reqwest::{Client, StatusCode};

pub async fn verify(client: &Client, base_url: &str) -> Result<()> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let url = format!("{}/", base_url);
    let resp = client
        .get(&url)
        .header("x-request-id", &request_id)
        .send()
        .await?;

    utils::verify_status(&resp, StatusCode::OK)?;

    let text = resp.text().await?;
    if text != "Hello, World!" {
        return Err(Error::VerificationFailed(format!(
            "Expected 'Hello, World!', got '{}'",
            text
        )));
    }
    Ok(())
}
