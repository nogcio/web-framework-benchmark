use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use std::{env, net::SocketAddr};
use mimalloc::MiMalloc;
use tokio::task;
use chrono::NaiveDateTime;


mod models;
mod schema;

use models::{Post, PostResponse, User, UserProfile};
use schema::{posts, users};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[derive(Clone)]
struct AppState {
    pool: DbPool,
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

    let manager = ConnectionManager::<PgConnection>::new(db_url);
    let pool_size = env::var("DB_POOL_SIZE")
        .unwrap_or_else(|_| "256".to_string())
        .parse::<u32>()
        .unwrap_or(256);

    let pool = r2d2::Pool::builder()
        .max_size(pool_size)
        .min_idle(Some(pool_size))
        .build(manager)
        .expect("Failed to create pool");

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
    let pool = state.pool.clone();
    
    let result = task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        diesel::sql_query("SELECT 1")
            .execute(&mut conn)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    })
    .await;

    match result {
        Ok(Ok(_)) => (StatusCode::OK, "OK"),
        Ok(Err(e)) => (e, "Database Error"),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Task Join Error"),
    }
}

async fn db_user_profile(
    State(state): State<AppState>,
    Path(email): Path<String>,
) -> impl IntoResponse {
    let pool_trending = state.pool.clone();
    let pool_user = state.pool.clone();

    // Task 1: Fetch Trending posts (views DESC limit 5)
    let trending_task = task::spawn_blocking(move || {
        let mut conn = pool_trending.get().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        posts::table
            .order(posts::views.desc())
            .limit(5)
            .select(Post::as_select())
            .load(&mut conn)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    });

    // Task 2: Fetch User, Update Login, Fetch User Posts
    let user_task = task::spawn_blocking(move || {
        let mut conn = pool_user.get().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // 1. Fetch User by email
        let user: User = users::table
            .filter(users::email.eq(&email))
            .select(User::as_select())
            .first(&mut conn)
            .optional()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        // 2. Update User last_login
        let last_login = diesel::update(users::table.find(user.id))
            .set(users::last_login.eq(diesel::dsl::now))
            .returning(users::last_login)
            .get_result::<Option<NaiveDateTime>>(&mut conn)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // 3. Fetch User Posts (created_at DESC limit 10)
        let user_posts: Vec<Post> = posts::table
            .filter(posts::user_id.eq(user.id))
            .order(posts::created_at.desc())
            .limit(10)
            .select(Post::as_select())
            .load(&mut conn)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok::<_, StatusCode>((user, last_login, user_posts))
    });

    let (trending_res, user_res) = tokio::join!(trending_task, user_task);

    let trending = match trending_res {
        Ok(Ok(t)) => t,
        Ok(Err(e)) => return e.into_response(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let (user, last_login, user_posts) = match user_res {
        Ok(Ok(u)) => u,
        Ok(Err(e)) => return e.into_response(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    Json(UserProfile {
        username: user.username,
        email: user.email,
        created_at: format!("{}Z", user.created_at),
        last_login: last_login.map(|t| format!("{}Z", t)),
        settings: user.settings,
        posts: user_posts.into_iter().map(PostResponse::from).collect(),
        trending: trending.into_iter().map(PostResponse::from).collect(),
    })
    .into_response()
}
