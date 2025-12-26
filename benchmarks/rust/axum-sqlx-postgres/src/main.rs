use axum::{
    extract::{FromRequestParts, Path, Query, Request, State},
    http::{header, request::Parts, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::NaiveDateTime;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{postgres::{PgConnectOptions, PgPoolOptions}, ConnectOptions, PgPool};
use std::{env, net::SocketAddr, str::FromStr, sync::OnceLock};
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

static DECODING_KEY: OnceLock<DecodingKey> = OnceLock::new();
static ENCODING_KEY: OnceLock<EncodingKey> = OnceLock::new();
static VALIDATION: OnceLock<Validation> = OnceLock::new();

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct HelloWorld {
    id: i32,
    name: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

#[tokio::main]
async fn main() {
    let port = env::var("PORT").unwrap_or_else(|_| "8000".to_string());
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

    let pool = PgPoolOptions::new()
        .max_connections(256)
        .min_connections(256)
        .test_before_acquire(false)
        .connect_with(connect_options)
        .await
        .expect("Failed to connect to database");

    let state = AppState { pool };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/db/read/one", get(db_read_one))
        .route("/db/read/many", get(db_read_many))
        .route("/db/write/insert", post(db_write_insert))
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/feed", get(get_feed))
        .route("/api/tweets", post(create_tweet))
        .route("/api/tweets/{id}", get(get_tweet))
        .route("/api/tweets/{id}/like", post(like_tweet))
        .layer(middleware::from_fn(x_request_id_middleware))
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

#[derive(Deserialize)]
struct ReadOneParams {
    id: i32,
}

async fn db_read_one(
    State(state): State<AppState>,
    Query(params): Query<ReadOneParams>,
) -> impl IntoResponse {
    let row = sqlx::query_as::<_, HelloWorld>(
        "SELECT id, name, created_at, updated_at FROM hello_world WHERE id = $1",
    )
        .bind(params.id)
        .fetch_one(&state.pool)
        .await;

    match row {
        Ok(row) => Json(row).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Deserialize)]
struct ReadManyParams {
    offset: Option<i32>,
    limit: Option<i32>,
}

async fn db_read_many(
    State(state): State<AppState>,
    Query(params): Query<ReadManyParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);

    let rows = sqlx::query_as::<_, HelloWorld>(
        "SELECT id, name, created_at, updated_at FROM hello_world ORDER BY id LIMIT $1 OFFSET $2",
    )
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await;

    match rows {
        Ok(rows) => Json(rows).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[derive(Deserialize)]
struct WriteInsertPayload {
    name: String,
}

async fn db_write_insert(
    State(state): State<AppState>,
    Json(payload): Json<WriteInsertPayload>,
) -> impl IntoResponse {
    let now = chrono::Utc::now().naive_utc();
    let row = sqlx::query_as::<_, HelloWorld>(
        "INSERT INTO hello_world (name, created_at, updated_at) VALUES ($1, $2, $3) RETURNING id, name, created_at, updated_at",
    )
    .bind(payload.name)
    .bind(now)
    .bind(now)
    .fetch_one(&state.pool)
    .await;

    match row {
        Ok(row) => Json(row).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn x_request_id_middleware(req: Request, next: Next) -> Response {
    let request_id = req.headers().get("x-request-id").cloned();
    let mut response = next.run(req).await;

    if let Some(request_id) = request_id {
        response.headers_mut().insert("x-request-id", request_id);
    }

    response
}

// --- Tweet Service ---

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: i32,
    name: String,
    exp: usize,
}

impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        if !auth_header.starts_with("Bearer ") {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let token = &auth_header[7..];
        let decoding_key = DECODING_KEY.get_or_init(|| DecodingKey::from_secret("secret".as_ref()));
        let token_data = decode::<Claims>(
            token,
            decoding_key,
            VALIDATION.get_or_init(Validation::default),
        )
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

        Ok(token_data.claims)
    }
}

#[derive(Deserialize)]
struct AuthRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    token: String,
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> impl IntoResponse {
    let mut hasher = Sha256::new();
    hasher.update(payload.password.as_bytes());
    let password_hash = hex::encode(hasher.finalize());

    let result = sqlx::query("INSERT INTO users (username, password_hash) VALUES ($1, $2)")
        .bind(payload.username)
        .bind(password_hash)
        .execute(&state.pool)
        .await;

    match result {
        Ok(_) => StatusCode::CREATED.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> impl IntoResponse {
    let mut hasher = Sha256::new();
    hasher.update(payload.password.as_bytes());
    let password_hash = hex::encode(hasher.finalize());

    let user = sqlx::query_as::<_, (i32, String)>("SELECT id, username FROM users WHERE username = $1 AND password_hash = $2")
        .bind(payload.username)
        .bind(password_hash)
        .fetch_optional(&state.pool)
        .await;

    match user {
        Ok(Some((id, username))) => {
            let claims = Claims {
                sub: id,
                name: username,
                exp: 10000000000, // Far future
            };

            let encoding_key = ENCODING_KEY.get_or_init(|| EncodingKey::from_secret("secret".as_ref()));
            let token = encode(
                &Header::default(),
                &claims,
                encoding_key,
            )
            .unwrap();

            Json(LoginResponse { token }).into_response()
        }
        Ok(None) => StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct Tweet {
    id: i32,
    username: String,
    content: String,
    created_at: NaiveDateTime,
    likes: i64,
}

async fn get_feed(State(state): State<AppState>, _claims: Claims) -> impl IntoResponse {
    let tweets = sqlx::query_as::<_, Tweet>(
        r#"
        SELECT t.id, u.username, t.content, t.created_at, (SELECT COUNT(*) FROM likes l WHERE l.tweet_id = t.id) as likes
        FROM tweets t
        JOIN users u ON t.user_id = u.id
        ORDER BY t.created_at DESC
        LIMIT 20
        "#,
    )
    .fetch_all(&state.pool)
    .await;

    match tweets {
        Ok(tweets) => Json(tweets).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn get_tweet(
    State(state): State<AppState>,
    _claims: Claims,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    let tweet = sqlx::query_as::<_, Tweet>(
        r#"
        SELECT t.id, u.username, t.content, t.created_at, (SELECT COUNT(*) FROM likes l WHERE l.tweet_id = t.id) as likes
        FROM tweets t
        JOIN users u ON t.user_id = u.id
        WHERE t.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await;

    match tweet {
        Ok(Some(tweet)) => Json(tweet).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[derive(Deserialize)]
struct CreateTweetRequest {
    content: String,
}

async fn create_tweet(
    State(state): State<AppState>,
    claims: Claims,
    Json(payload): Json<CreateTweetRequest>,
) -> impl IntoResponse {
    let result = sqlx::query("INSERT INTO tweets (user_id, content) VALUES ($1, $2)")
        .bind(claims.sub)
        .bind(payload.content)
        .execute(&state.pool)
        .await;

    match result {
        Ok(_) => StatusCode::CREATED.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn like_tweet(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM likes WHERE user_id = $1 AND tweet_id = $2")
        .bind(claims.sub)
        .bind(id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                StatusCode::OK.into_response()
            } else {
                let insert_result = sqlx::query("INSERT INTO likes (user_id, tweet_id) VALUES ($1, $2)")
                    .bind(claims.sub)
                    .bind(id)
                    .execute(&state.pool)
                    .await;
                match insert_result {
                    Ok(_) => StatusCode::OK.into_response(),
                    Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                }
            }
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
