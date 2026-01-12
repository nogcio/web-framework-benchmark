use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::{PgConnectOptions, PgPoolOptions}, ConnectOptions, PgPool};
use std::{env, net::SocketAddr, str::FromStr};
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

#[derive(Serialize, sqlx::FromRow)]
struct User {
    id: i32,
    username: String,
    email: String,
    created_at: NaiveDateTime,
    last_login: Option<NaiveDateTime>,
    settings: serde_json::Value,
}

#[derive(Serialize, sqlx::FromRow)]
struct Post {
    id: i32,
    title: String,
    content: String,
    views: i32,
    created_at: NaiveDateTime,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UserProfile {
    username: String,
    email: String,
    created_at: String,
    last_login: Option<String>,
    settings: serde_json::Value,
    posts: Vec<PostResponse>,
    trending: Vec<PostResponse>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PostResponse {
    id: i32,
    title: String,
    content: String,
    views: i32,
    created_at: String,
}

impl From<Post> for PostResponse {
    fn from(post: Post) -> Self {
        Self {
            id: post.id,
            title: post.title,
            content: post.content,
            views: post.views,
            created_at: format!("{}Z", post.created_at),
        }
    }
}

#[tokio::main]
async fn main() {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port).parse::<SocketAddr>().unwrap();

    let db_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        env::var("DB_USER").unwrap_or_else(|_| "benchmark".to_string()),
        env::var("DB_PASSWORD").unwrap_or_else(|_| "benchmark".to_string()),
        env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
        env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string()),
        env::var("DB_NAME").unwrap_or_else(|_| "benchmark".to_string())
    );

    let connect_options = PgConnectOptions::from_str(&db_url)
        .expect("Invalid connection string")
        .log_statements(log::LevelFilter::Off);

    let pool_size = env::var("DB_POOL_SIZE")
        .unwrap_or_else(|_| "256".to_string())
        .parse::<u32>()
        .unwrap_or(256);

    let pool = PgPoolOptions::new()
        .max_connections(pool_size)
        .min_connections(pool_size)
        .test_before_acquire(false)
        .connect_with(connect_options)
        .await
        .expect("Failed to connect to database");

    let state = AppState { pool };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/db/user-profile/{email}", get(db_user_profile))
        .with_state(state);

    println!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query("SELECT 1").execute(&state.pool).await {
        Ok(_) => (StatusCode::OK, "OK"),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database Error"),
    }
}

async fn get_user_profile_logic(pool: &PgPool, email: String) -> Result<UserProfile, StatusCode> {
    let user_query = sqlx::query_as::<_, User>(
        "SELECT id, username, email, created_at, last_login, settings FROM users WHERE email = $1",
    )
    .bind(&email)
    .fetch_optional(pool);

    let trending_query = sqlx::query_as::<_, Post>(
        "SELECT id, title, content, views, created_at FROM posts ORDER BY views DESC LIMIT 5",
    )
    .fetch_all(pool);

    let (user_result, trending_result) = tokio::join!(user_query, trending_query);

    let user = match user_result {
        Ok(Some(user)) => user,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let trending = match trending_result {
        Ok(posts) => posts,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let update_query = sqlx::query_scalar::<_, Option<NaiveDateTime>>(
        "UPDATE users SET last_login = NOW() WHERE id = $1 RETURNING last_login",
    )
    .bind(user.id)
    .fetch_one(pool);

    let posts_query = sqlx::query_as::<_, Post>(
        "SELECT id, title, content, views, created_at FROM posts WHERE user_id = $1 ORDER BY created_at DESC LIMIT 10",
    )
    .bind(user.id)
    .fetch_all(pool);

    let (last_login_result, posts_result) = tokio::join!(update_query, posts_query);

    let last_login = last_login_result.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let posts = posts_result.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(UserProfile {
        username: user.username,
        email: user.email,
        created_at: format!("{}Z", user.created_at),
        last_login: last_login.map(|t| format!("{}Z", t)),
        settings: user.settings,
        posts: posts.into_iter().map(PostResponse::from).collect(),
        trending: trending.into_iter().map(PostResponse::from).collect(),
    })
}

async fn db_user_profile(
    State(state): State<AppState>,
    Path(email): Path<String>,
) -> impl IntoResponse {
    match get_user_profile_logic(&state.pool, email).await {
        Ok(profile) => Json(profile).into_response(),
        Err(status) => status.into_response(),
    }
}
