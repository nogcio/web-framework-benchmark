use crate::prelude::*;
use crate::verification::utils;
use reqwest::{Client, StatusCode};
use serde_json::Value;

pub async fn verify(client: &Client, base_url: &str) -> Result<()> {
    let suffix = uuid::Uuid::new_v4().to_string();
    let username = format!("user_{}", suffix);
    let password = "password123";

    // 1. Register
    let request_id = uuid::Uuid::new_v4().to_string();
    let url = format!("{}/api/auth/register", base_url);
    let resp = client
        .post(&url)
        .header("x-request-id", &request_id)
        .json(&serde_json::json!({ "username": username, "password": password }))
        .send()
        .await?;

    utils::verify_status(&resp, StatusCode::CREATED)?;
    utils::verify_request_id(&resp, &request_id)?;

    // 2. Login (Success)
    let request_id = uuid::Uuid::new_v4().to_string();
    let url = format!("{}/api/auth/login", base_url);
    let resp = client
        .post(&url)
        .header("x-request-id", &request_id)
        .json(&serde_json::json!({ "username": username, "password": password }))
        .send()
        .await?;

    utils::verify_status(&resp, StatusCode::OK)?;
    utils::verify_request_id(&resp, &request_id)?;

    let json: Value = resp.json().await?;
    utils::verify_no_extra_fields(&json, &["token"])?;

    let token = json["token"].as_str().ok_or_else(|| {
        Error::VerificationFailed(format!("Login response missing token: {:?}", json))
    })?;

    // 3. Login (Failure - Wrong Password)
    let request_id = uuid::Uuid::new_v4().to_string();
    let url = format!("{}/api/auth/login", base_url);
    let resp = client
        .post(&url)
        .header("x-request-id", &request_id)
        .json(&serde_json::json!({ "username": username, "password": "wrongpassword" }))
        .send()
        .await?;

    utils::verify_status(&resp, StatusCode::UNAUTHORIZED)?;
    utils::verify_request_id(&resp, &request_id)?;

    // 4. Unauthorized Access Checks
    // 4a. Feed without token
    let request_id = uuid::Uuid::new_v4().to_string();
    let url = format!("{}/api/feed", base_url);
    let resp = client
        .get(&url)
        .header("x-request-id", &request_id)
        .send()
        .await?;

    utils::verify_status(&resp, StatusCode::UNAUTHORIZED)?;
    utils::verify_request_id(&resp, &request_id)?;

    // 4b. Create Tweet without token
    let request_id = uuid::Uuid::new_v4().to_string();
    let url = format!("{}/api/tweets", base_url);
    let resp = client
        .post(&url)
        .header("x-request-id", &request_id)
        .json(&serde_json::json!({ "content": "unauthorized content" }))
        .send()
        .await?;

    utils::verify_status(&resp, StatusCode::UNAUTHORIZED)?;
    utils::verify_request_id(&resp, &request_id)?;

    // 4c. Get Tweet without token (using dummy ID 1)
    let request_id = uuid::Uuid::new_v4().to_string();
    let url = format!("{}/api/tweets/1", base_url);
    let resp = client
        .get(&url)
        .header("x-request-id", &request_id)
        .send()
        .await?;

    utils::verify_status(&resp, StatusCode::UNAUTHORIZED)?;
    utils::verify_request_id(&resp, &request_id)?;

    // 4d. Like Tweet without token (using dummy ID 1)
    let request_id = uuid::Uuid::new_v4().to_string();
    let url = format!("{}/api/tweets/1/like", base_url);
    let resp = client
        .post(&url)
        .header("x-request-id", &request_id)
        .send()
        .await?;

    utils::verify_status(&resp, StatusCode::UNAUTHORIZED)?;
    utils::verify_request_id(&resp, &request_id)?;

    // 5. Create Tweet
    let request_id = uuid::Uuid::new_v4().to_string();
    let url = format!("{}/api/tweets", base_url);
    let content = format!("Hello world {}", suffix);
    let resp = client
        .post(&url)
        .header("x-request-id", &request_id)
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "content": content }))
        .send()
        .await?;

    utils::verify_status(&resp, StatusCode::CREATED)?;
    utils::verify_request_id(&resp, &request_id)?;
    // Note: Implementation returns 201 with empty body, so we don't parse JSON here.

    // 6. Read Feed (and find the created tweet)
    let request_id = uuid::Uuid::new_v4().to_string();
    let url = format!("{}/api/feed", base_url);
    let resp = client
        .get(&url)
        .header("x-request-id", &request_id)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    utils::verify_status(&resp, StatusCode::OK)?;
    utils::verify_request_id(&resp, &request_id)?;

    let feed: Value = resp.json().await?;
    let feed_arr = feed
        .as_array()
        .ok_or_else(|| Error::VerificationFailed(format!("Feed is not an array: {:?}", feed)))?;

    let mut tweet_id: Option<String> = None;
    for tweet in feed_arr {
        utils::verify_no_extra_fields(tweet, &["id", "username", "content", "createdAt", "likes"])?;

        if tweet["content"].as_str() == Some(&content) {
            // Verify fields
            let id_val = &tweet["id"];
            if id_val.as_i64().is_none() && id_val.as_u64().is_none() && id_val.as_str().is_none() {
                return Err(Error::VerificationFailed(
                    "Tweet in feed missing 'id' (must be int or string)".to_string(),
                ));
            }
            if tweet["username"].as_str() != Some(&username) {
                return Err(Error::VerificationFailed(format!(
                    "Tweet username mismatch. Expected {}, got {:?}",
                    username, tweet["username"]
                )));
            }
            if let Some(s) = tweet["createdAt"].as_str() {
                utils::verify_iso_date(s, "createdAt")?;
            } else {
                return Err(Error::VerificationFailed(
                    "Tweet in feed missing 'createdAt'".to_string(),
                ));
            }
            if tweet["likes"].as_i64().is_none() && tweet["likes"].as_u64().is_none() {
                return Err(Error::VerificationFailed(
                    "Tweet in feed missing 'likes'".to_string(),
                ));
            }

            tweet_id = if let Some(s) = id_val.as_str() {
                Some(s.to_string())
            } else {
                id_val
                    .as_i64()
                    .map(|i| i.to_string())
                    .or(id_val.as_u64().map(|u| u.to_string()))
            };
            break;
        }
    }

    let tweet_id = tweet_id.ok_or_else(|| {
        Error::VerificationFailed(format!(
            "Could not find created tweet with content '{}' in feed",
            content
        ))
    })?;

    // 7. Read Tweet
    let request_id = uuid::Uuid::new_v4().to_string();
    let url = format!("{}/api/tweets/{}", base_url, tweet_id);
    let resp = client
        .get(&url)
        .header("x-request-id", &request_id)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    utils::verify_status(&resp, StatusCode::OK)?;
    utils::verify_request_id(&resp, &request_id)?;

    let tweet: Value = resp.json().await?;
    utils::verify_no_extra_fields(&tweet, &["id", "username", "content", "createdAt", "likes"])?;

    // Verify all fields
    // ID
    let id_match = if let Some(s) = tweet["id"].as_str() {
        s == tweet_id
    } else if let Some(i) = tweet["id"].as_i64() {
        i.to_string() == tweet_id
    } else if let Some(u) = tweet["id"].as_u64() {
        u.to_string() == tweet_id
    } else {
        false
    };

    if !id_match {
        return Err(Error::VerificationFailed(format!(
            "Expected tweet id {}, got {:?}",
            tweet_id, tweet["id"]
        )));
    }

    // Username
    if tweet["username"].as_str() != Some(&username) {
        return Err(Error::VerificationFailed(format!(
            "Expected username '{}', got {:?}",
            username, tweet["username"]
        )));
    }

    // Content
    if tweet["content"].as_str() != Some(&content) {
        return Err(Error::VerificationFailed(format!(
            "Read tweet content mismatch. Expected '{}', got {:?}",
            content, tweet["content"]
        )));
    }

    // CreatedAt
    if let Some(s) = tweet["createdAt"].as_str() {
        utils::verify_iso_date(s, "createdAt")?;
    } else {
        return Err(Error::VerificationFailed(
            "Tweet missing 'createdAt'".to_string(),
        ));
    }

    // Likes
    if tweet["likes"].as_i64() != Some(0) && tweet["likes"].as_u64() != Some(0) {
        return Err(Error::VerificationFailed(format!(
            "Expected 0 likes for new tweet, got {:?}",
            tweet["likes"]
        )));
    }

    // 8. Like Tweet (Toggle On)
    let request_id = uuid::Uuid::new_v4().to_string();
    let url = format!("{}/api/tweets/{}/like", base_url, tweet_id);
    let resp = client
        .post(&url)
        .header("x-request-id", &request_id)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    utils::verify_status(&resp, StatusCode::OK)?;
    utils::verify_request_id(&resp, &request_id)?;

    // Verify likes count increased
    let url = format!("{}/api/tweets/{}", base_url, tweet_id);
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
    let tweet: Value = resp.json().await?;
    utils::verify_no_extra_fields(&tweet, &["id", "username", "content", "createdAt", "likes"])?;

    if tweet["likes"].as_i64() != Some(1) && tweet["likes"].as_u64() != Some(1) {
        return Err(Error::VerificationFailed(format!(
            "Expected 1 like after liking, got {:?}",
            tweet["likes"]
        )));
    }

    // 9. Like Tweet (Toggle Off)
    let request_id = uuid::Uuid::new_v4().to_string();
    let url = format!("{}/api/tweets/{}/like", base_url, tweet_id);
    let resp = client
        .post(&url)
        .header("x-request-id", &request_id)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    utils::verify_status(&resp, StatusCode::OK)?;
    utils::verify_request_id(&resp, &request_id)?;

    // Verify likes count decreased
    let url = format!("{}/api/tweets/{}", base_url, tweet_id);
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
    let tweet: Value = resp.json().await?;
    utils::verify_no_extra_fields(&tweet, &["id", "username", "content", "createdAt", "likes"])?;

    if tweet["likes"].as_i64() != Some(0) && tweet["likes"].as_u64() != Some(0) {
        return Err(Error::VerificationFailed(format!(
            "Expected 0 likes after unliking, got {:?}",
            tweet["likes"]
        )));
    }

    Ok(())
}
