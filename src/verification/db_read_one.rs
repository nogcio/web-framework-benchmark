use crate::database::DatabaseKind;
use crate::prelude::*;
use crate::verification::utils;
use reqwest::{Client, StatusCode};
use serde_json::Value;

pub async fn verify(client: &Client, base_url: &str, database: Option<DatabaseKind>) -> Result<()> {
    let id_param = match database {
        Some(DatabaseKind::Mongodb) => "000000000000000000000001",
        _ => "1",
    };
    let request_id = uuid::Uuid::new_v4().to_string();
    let url = format!("{}/db/read/one?id={}", base_url, id_param);
    let resp = client
        .get(&url)
        .header("x-request-id", &request_id)
        .send()
        .await?;

    utils::verify_status(&resp, StatusCode::OK)?;
    utils::verify_request_id(&resp, &request_id)?;

    let json: Value = resp.json().await?;

    utils::verify_no_extra_fields(&json, &["id", "name", "createdAt", "updatedAt"])?;

    // Check ID
    let id_match = match database {
        Some(DatabaseKind::Mongodb) => json["id"].as_str() == Some(id_param),
        _ => json["id"].as_u64() == Some(1) || json["id"].as_i64() == Some(1),
    };

    if !id_match {
        return Err(Error::VerificationFailed(format!(
            "Expected id {}, got {:?}",
            id_param, json["id"]
        )));
    }

    // Check Name
    let expected_name = "name_1";
    if json["name"] != expected_name {
        return Err(Error::VerificationFailed(format!(
            "Expected name '{}', got {:?}",
            expected_name, json["name"]
        )));
    }

    // Check createdAt
    if let Some(s) = json["createdAt"].as_str() {
        utils::verify_iso_date(s, "createdAt")?;
    } else {
        return Err(Error::VerificationFailed(
            "Expected 'createdAt' field to be a string".to_string(),
        ));
    }

    // Check updatedAt
    if let Some(s) = json["updatedAt"].as_str() {
        utils::verify_iso_date(s, "updatedAt")?;
    } else {
        return Err(Error::VerificationFailed(
            "Expected 'updatedAt' field to be a string".to_string(),
        ));
    }

    Ok(())
}
