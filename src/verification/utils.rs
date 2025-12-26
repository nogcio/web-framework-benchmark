use crate::prelude::*;
use reqwest::{Response, StatusCode};

pub fn verify_status(resp: &Response, expected: StatusCode) -> Result<()> {
    if resp.status() != expected {
        return Err(Error::VerificationFailed(format!(
            "Expected {} {}, got {}",
            expected.as_u16(),
            expected.canonical_reason().unwrap_or(""),
            resp.status()
        )));
    }
    Ok(())
}

pub fn verify_request_id(resp: &Response, expected_id: &str) -> Result<()> {
    if let Some(val) = resp.headers().get("x-request-id") {
        if val != expected_id {
            return Err(Error::VerificationFailed(format!(
                "Expected x-request-id '{}', got {:?}",
                expected_id, val
            )));
        }
    } else {
        return Err(Error::VerificationFailed(
            "Response missing x-request-id header".to_string(),
        ));
    }
    Ok(())
}

pub fn verify_iso_date(s: &str, field_name: &str) -> Result<()> {
    if chrono::DateTime::parse_from_rfc3339(s).is_err()
        && chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").is_err()
        && chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f").is_err()
    {
        return Err(Error::VerificationFailed(format!(
            "'{}' is not a valid ISO 8601 date: {}",
            field_name, s
        )));
    }
    Ok(())
}

pub fn verify_no_extra_fields(json: &serde_json::Value, expected_fields: &[&str]) -> Result<()> {
    if let Some(obj) = json.as_object() {
        for (key, _) in obj {
            if !expected_fields.contains(&key.as_str()) {
                return Err(Error::VerificationFailed(format!(
                    "Unexpected field '{}' in response",
                    key
                )));
            }
        }
    } else {
        return Err(Error::VerificationFailed(
            "Expected JSON object".to_string(),
        ));
    }
    Ok(())
}
