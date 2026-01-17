use anyhow::Context;
use askama::Template;
use axum::extract::State;
use axum::response::IntoResponse;
use reqwest::header::{ACCEPT, USER_AGENT};
use serde::Deserialize;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::warn;

use super::render::HtmlTemplate;
use crate::state::AppState;

#[allow(unused_imports)]
use crate::filters;

const CACHE_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone, Default)]
struct StarsCache {
    value: Option<u64>,
    fetched_at: Option<Instant>,
}

static STARS_CACHE: OnceLock<RwLock<StarsCache>> = OnceLock::new();

fn stars_cache() -> &'static RwLock<StarsCache> {
    STARS_CACHE.get_or_init(|| RwLock::new(StarsCache::default()))
}

#[derive(Debug, Deserialize)]
struct GithubRepoResponse {
    stargazers_count: u64,
}

#[derive(Template)]
#[template(path = "components/header/github-stars.rs.j2")]
struct GithubStarsTemplate {
    value: String,
}

pub async fn github_stars_partials_handler(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    HtmlTemplate(GithubStarsTemplate {
        value: github_stars_value_string().await,
    })
}

pub(super) async fn github_stars_value_string() -> String {
    match get_github_stars_cached().await.unwrap_or(None) {
        Some(value) => value.to_string(),
        None => "—".to_string(),
    }
}

pub(super) async fn get_github_stars_cached() -> anyhow::Result<Option<u64>> {
    {
        let cache = stars_cache().read().await;
        if let Some(fetched_at) = cache.fetched_at
            && fetched_at.elapsed() < CACHE_TTL
        {
            return Ok(cache.value);
        }
    }

    let mut cache = stars_cache().write().await;

    if let Some(fetched_at) = cache.fetched_at
        && fetched_at.elapsed() < CACHE_TTL
    {
        return Ok(cache.value);
    }

    let fetched_at = Instant::now();
    match fetch_github_stars().await {
        Ok(value) => {
            cache.value = value;
            cache.fetched_at = Some(fetched_at);
            Ok(value)
        }
        Err(err) => {
            warn!(error = %err, "failed to fetch GitHub stars; serving cached value");
            cache.fetched_at = Some(fetched_at);
            Ok(cache.value)
        }
    }
}

async fn fetch_github_stars() -> anyhow::Result<Option<u64>> {
    let (owner, repo) = parse_github_owner_repo(crate::handlers::web::types::REPOSITORY_URL)
        .context("failed to parse GitHub owner/repo from repository URL")?;

    let api_url = format!("https://api.github.com/repos/{owner}/{repo}");

    let client = reqwest::Client::new();
    let mut req = client
        .get(api_url)
        .header(USER_AGENT, "wfb-server")
        .header(ACCEPT, "application/vnd.github+json");

    if let Ok(token) = std::env::var("WFB_GITHUB_TOKEN").or_else(|_| std::env::var("GITHUB_TOKEN"))
        && !token.trim().is_empty()
    {
        req = req.bearer_auth(token);
    }

    let resp = req.send().await.context("GitHub API request failed")?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    let data: GithubRepoResponse = resp
        .json()
        .await
        .context("GitHub API response decode failed")?;

    Ok(Some(data.stargazers_count))
}

fn parse_github_owner_repo(url: &str) -> Option<(String, String)> {
    let trimmed = url.trim().trim_end_matches('/');
    let without_git = trimmed.strip_suffix(".git").unwrap_or(trimmed);

    let without_scheme = without_git
        .strip_prefix("https://")
        .or_else(|| without_git.strip_prefix("http://"))
        .unwrap_or(without_git);

    let without_host = without_scheme.strip_prefix("github.com/")?;

    let mut parts = without_host.split('/');
    let owner = parts.next()?.trim();
    let repo = parts.next()?.trim();

    if owner.is_empty() || repo.is_empty() {
        return None;
    }

    Some((owner.to_string(), repo.to_string()))
}
