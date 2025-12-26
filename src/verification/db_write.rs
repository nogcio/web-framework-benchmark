use crate::prelude::*;
use crate::verification::utils;
use reqwest::{Client, StatusCode};
use serde_json::Value;

pub async fn verify(client: &Client, base_url: &str) -> Result<()> {
    let name = format!("verify_{}", uuid::Uuid::new_v4());
    let request_id = uuid::Uuid::new_v4().to_string();
    let url = format!("{}/db/write/insert", base_url);
    let resp = client
        .post(&url)
        .header("x-request-id", &request_id)
        .json(&serde_json::json!({ "name": name }))
        .send()
        .await?;

    utils::verify_status(&resp, StatusCode::OK)?;
    utils::verify_request_id(&resp, &request_id)?;

    let json: Value = resp.json().await?;
    utils::verify_no_extra_fields(&json, &["id", "name", "createdAt", "updatedAt"])?;

    if json["name"] != name {
        return Err(Error::VerificationFailed(format!(
            "Expected name '{}', got {:?}",
            name, json["name"]
        )));
    }
    // Should have an ID
    if json["id"].is_null() {
        return Err(Error::VerificationFailed(
            "Expected 'id' field in response".to_string(),
        ));
    }
    // Should have createdAt
    if let Some(s) = json["createdAt"].as_str() {
        utils::verify_iso_date(s, "createdAt")?;
    } else {
        return Err(Error::VerificationFailed(
            "Expected 'createdAt' field to be a string".to_string(),
        ));
    }
    // Should have updatedAt
    if let Some(s) = json["updatedAt"].as_str() {
        utils::verify_iso_date(s, "updatedAt")?;
    } else {
        return Err(Error::VerificationFailed(
            "Expected 'updatedAt' field to be a string".to_string(),
        ));
    }
    Ok(())
}
