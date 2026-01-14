use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use std::env;
use mimalloc::MiMalloc;
use chrono::NaiveDateTime;

mod models;
mod schema;

use models::{Post, PostResponse, User, UserProfile};
use schema::{posts, users};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

struct AppState {
    pool: DbPool,
}

#[get("/health")]
async fn health_check(data: web::Data<AppState>) -> impl Responder {
    let pool = data.pool.clone();
    let result = web::block(move || {
        let mut conn = pool.get().map_err(|_| "Pool Error")?;
        diesel::sql_query("SELECT 1")
            .execute(&mut conn)
            .map_err(|_| "Database Error")
    })
    .await;

    match result {
        Ok(Ok(_)) => HttpResponse::Ok().body("OK"),
        _ => HttpResponse::InternalServerError().body("Database Error"),
    }
}

#[get("/db/user-profile/{email}")]
async fn db_user_profile(
    data: web::Data<AppState>,
    email: web::Path<String>,
) -> impl Responder {
    let pool = data.pool.clone();
    let email_str = email.into_inner();

    // Task 1: Fetch Trending posts (Parallel Phase 1)
    let pool_trending = pool.clone();
    let trending_task = web::block(move || {
        let mut conn = pool_trending.get().map_err(|_| "Pool error")?;
        posts::table
            .order(posts::views.desc())
            .limit(5)
            .select(Post::as_select())
            .load::<Post>(&mut conn)
            .map_err(|_| "Query error")
    });

    // Task 2: Fetch User (Parallel Phase 1)
    let pool_user = pool.clone();
    let email_clone = email_str.clone();
    let user_task = web::block(move || {
        let mut conn = pool_user.get().map_err(|_| "Pool error")?;
        users::table
            .filter(users::email.eq(&email_clone))
            .select(User::as_select())
            .first::<User>(&mut conn)
            .optional()
            .map_err(|_| "Query error")
    });

    let (trending_res, user_res) = tokio::join!(trending_task, user_task);

    let trending = match trending_res {
        Ok(Ok(t)) => t,
        _ => return HttpResponse::InternalServerError().finish(),
    };

    let user = match user_res {
        Ok(Ok(Some(u))) => u,
        Ok(Ok(None)) => return HttpResponse::NotFound().finish(),
        _ => return HttpResponse::InternalServerError().finish(),
    };

    // Task 3: Update Last Login (Parallel Phase 2)
    let pool_update = pool.clone();
    let user_id = user.id;
    let update_task = web::block(move || {
        let mut conn = pool_update.get().map_err(|_| "Pool error")?;
        diesel::update(users::table.find(user_id))
            .set(users::last_login.eq(diesel::dsl::now))
            .returning(users::last_login)
            .get_result::<Option<NaiveDateTime>>(&mut conn)
            .map_err(|_| "Update error")
    });

    // Task 4: Fetch User Posts (Parallel Phase 2)
    let pool_posts = pool.clone();
    let posts_task = web::block(move || {
        let mut conn = pool_posts.get().map_err(|_| "Pool error")?;
        posts::table
            .filter(posts::user_id.eq(user_id))
            .order(posts::created_at.desc())
            .limit(10)
            .select(Post::as_select())
            .load::<Post>(&mut conn)
            .map_err(|_| "Query error")
    });

    let (update_res, posts_res) = tokio::join!(update_task, posts_task);
    
    let last_login = match update_res {
        Ok(Ok(l)) => l,
        _ => return HttpResponse::InternalServerError().finish(),
    };
    
    let user_posts = match posts_res {
        Ok(Ok(p)) => p,
        _ => return HttpResponse::InternalServerError().finish(),
    };

    HttpResponse::Ok().json(UserProfile {
        username: user.username,
        email: user.email,
        created_at: format!("{}Z", user.created_at),
        last_login: last_login.map(|t| format!("{}Z", t)),
        settings: user.settings,
        posts: user_posts.into_iter().map(PostResponse::from).collect(),
        trending: trending.into_iter().map(PostResponse::from).collect(),
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);

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

    println!("Listening on {}", addr);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState { pool: pool.clone() }))
            .service(health_check)
            .service(db_user_profile)
    })
    .bind(addr)?
    .run()
    .await
}
