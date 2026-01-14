from contextlib import asynccontextmanager
from fastapi import FastAPI, Request, Response, HTTPException, Depends
from fastapi.responses import PlainTextResponse, ORJSONResponse
import asyncpg
import os
import asyncio
import json

pool = None

@asynccontextmanager
async def lifespan(app: FastAPI):
    global pool
    pool_size = int(os.getenv("DB_POOL_SIZE", "256"))
    pool = await asyncpg.create_pool(
        user=os.getenv("DB_USER", "benchmark"),
        password=os.getenv("DB_PASSWORD", "benchmark"),
        database=os.getenv("DB_NAME", "benchmark"),
        host=os.getenv("DB_HOST", "localhost"),
        port=os.getenv("DB_PORT", "5432"),
        min_size=pool_size,
        max_size=pool_size,
        statement_cache_size=1024,
    )
    yield
    await pool.close()

app = FastAPI(lifespan=lifespan, default_response_class=ORJSONResponse, openapi_url=None, docs_url=None, redoc_url=None)

@app.get("/health")
async def health():
    try:
        async with pool.acquire() as conn:
            await conn.execute("SELECT 1")
        return PlainTextResponse("OK")
    except Exception:
        return Response(status_code=500, content="Database unavailable")

async def fetch_user_by_email(conn, email: str):
    return await conn.fetchrow(
        'SELECT id, username, email, created_at, last_login, settings FROM users WHERE email = $1',
        email
    )

async def update_user_last_login(conn, user_id: int):
    return await conn.fetchval(
        'UPDATE users SET last_login = NOW() WHERE id = $1 RETURNING last_login',
        user_id
    )

async def fetch_trending_posts(conn):
    return await conn.fetch(
        'SELECT id, title, content, views, created_at FROM posts ORDER BY views DESC LIMIT 5'
    )

async def fetch_user_posts(conn, user_id: int):
    return await conn.fetch(
        'SELECT id, title, content, views, created_at FROM posts WHERE user_id = $1 ORDER BY created_at DESC LIMIT 10',
        user_id
    )

def map_post(row):
    return {
        "id": row["id"],
        "title": row["title"],
        "content": row["content"],
        "views": row["views"],
        "createdAt": row["created_at"].isoformat() + "Z" if row["created_at"] else None
    }

async def get_user_profile_logic(email: str):
    # Acquire two connections to enable parallelism (mimicking Node.js/Go behavior)
    async with pool.acquire() as conn1, pool.acquire() as conn2:
        # Parallel Execution: Query A (User) and Query B (Trending)
        user_row, trending_rows = await asyncio.gather(
            fetch_user_by_email(conn1, email),
            fetch_trending_posts(conn2)
        )
        
        if not user_row:
            raise HTTPException(status_code=404, detail="User not found")
            
        # Parallel Execution: Query C (Posts) and Query D (Update User)
        posts_rows, last_login = await asyncio.gather(
            fetch_user_posts(conn1, user_row["id"]),
            update_user_last_login(conn2, user_row["id"])
        )
        
        # Handle settings (asyncpg might return str or dict depending on column type and codec)
        settings = user_row["settings"]
        if isinstance(settings, str):
            settings = json.loads(settings)
        
        return {
            "username": user_row["username"],
            "email": user_row["email"],
            "createdAt": user_row["created_at"].isoformat() + "Z" if user_row["created_at"] else None,
            "lastLogin": last_login.isoformat() + "Z" if last_login else None,
            "settings": settings,
            "posts": [map_post(row) for row in posts_rows],
            "trending": [map_post(row) for row in trending_rows]
        }

@app.get("/db/user-profile/{email}")
async def db_user_profile(email: str):
    return await get_user_profile_logic(email)
