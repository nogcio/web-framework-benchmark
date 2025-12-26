use crate::database::DatabaseKind;
use crate::prelude::*;
use crate::verification::utils;
use reqwest::{Client, StatusCode};
use serde_json::Value;

pub async fn verify(client: &Client, base_url: &str, database: Option<DatabaseKind>) -> Result<()> {
    let limit = 10;
    let request_id = uuid::Uuid::new_v4().to_string();
    let url = format!("{}/db/read/many?offset=0&limit={}", base_url, limit);
    let resp = client
        .get(&url)
        .header("x-request-id", &request_id)
        .send()
        .await?;

    utils::verify_status(&resp, StatusCode::OK)?;
    utils::verify_request_id(&resp, &request_id)?;

    let json: Value = resp.json().await?;
    let arr = json
        .as_array()
        .ok_or_else(|| Error::VerificationFailed(format!("Expected json array, got {:?}", json)))?;

    if arr.len() != limit {
        return Err(Error::VerificationFailed(format!(
            "Expected {} items, got {}",
            limit,
            arr.len()
        )));
    }

    for (i, item) in arr.iter().enumerate() {
        utils::verify_no_extra_fields(item, &["id", "name", "createdAt", "updatedAt"])?;

        let expected_idx = i + 1;

        let id_match = match database {
            Some(DatabaseKind::Mongodb) => {
                let expected_hex = format!("{:024x}", expected_idx);
                item["id"].as_str() == Some(&expected_hex)
            }
            _ => {
                item["id"].as_u64() == Some(expected_idx as u64)
                    || item["id"].as_i64() == Some(expected_idx as i64)
            }
        };

        if !id_match {
            return Err(Error::VerificationFailed(format!(
                "Item at index {}: Expected id corresponding to {}, got {:?}",
                i, expected_idx, item["id"]
            )));
        }

        let expected_name = format!("name_{}", expected_idx);
        if item["name"] != expected_name {
            return Err(Error::VerificationFailed(format!(
                "Item at index {}: Expected name '{}', got {:?}",
                i, expected_name, item["name"]
            )));
        }

        // Check createdAt
        if let Some(s) = item["createdAt"].as_str() {
            utils::verify_iso_date(s, &format!("Item at index {}: createdAt", i))?;
        } else {
            return Err(Error::VerificationFailed(format!(
                "Item at index {}: Expected 'createdAt' field to be a string",
                i
            )));
        }

        // Check updatedAt
        if let Some(s) = item["updatedAt"].as_str() {
            utils::verify_iso_date(s, &format!("Item at index {}: updatedAt", i))?;
        } else {
            return Err(Error::VerificationFailed(format!(
                "Item at index {}: Expected 'updatedAt' field to be a string",
                i
            )));
        }
    }

    Ok(())
}
