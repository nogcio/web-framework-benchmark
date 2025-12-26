from contextlib import asynccontextmanager
from fastapi import FastAPI, Request, Response, HTTPException, Depends, status
from fastapi.responses import PlainTextResponse, JSONResponse
from pydantic import BaseModel
import asyncpg
import jwt
import hashlib
import os
from datetime import datetime, timedelta
from typing import List, Optional

JWT_SECRET = "benchmark-secret"

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
        # Cache prepared statements per-connection to reduce overhead on hot SQL.
        statement_cache_size=1024,
    )
    yield
    await pool.close()

app = FastAPI(lifespan=lifespan, openapi_url=None, docs_url=None, redoc_url=None)

@app.middleware("http")
async def x_request_id_middleware(request: Request, call_next):
    response = await call_next(request)
    request_id = request.headers.get("x-request-id")
    if request_id:
        response.headers["x-request-id"] = request_id
    return response

@app.get("/health")
async def health():
    try:
        async with pool.acquire() as conn:
            await conn.execute("SELECT 1")
        return PlainTextResponse("OK")
    except Exception:
        return Response(status_code=500, content="Database unavailable")

# --- Database Tests ---

@app.get("/db/read/one")
async def db_read_one(id: int):
    row = await pool.fetchrow('SELECT id, name, created_at as "createdAt", updated_at as "updatedAt" FROM hello_world WHERE id = $1', id)
    if not row:
        raise HTTPException(status_code=404)
    return dict(row)

@app.get("/db/read/many")
async def db_read_many(offset: int = 0, limit: int = 50):
    rows = await pool.fetch('SELECT id, name, created_at as "createdAt", updated_at as "updatedAt" FROM hello_world ORDER BY id LIMIT $1 OFFSET $2', limit, offset)
    return [dict(row) for row in rows]

class WriteInsertPayload(BaseModel):
    name: str

@app.post("/db/write/insert")
async def db_write_insert(payload: WriteInsertPayload):
    now = datetime.now()
    row = await pool.fetchrow(
        "INSERT INTO hello_world (name, created_at, updated_at) VALUES ($1, $2, $3) RETURNING id, name, created_at as \"createdAt\", updated_at as \"updatedAt\"",
        payload.name, now, now
    )
    return dict(row)

# --- Tweet Service ---

class AuthRequest(BaseModel):
    username: str
    password: str

class CreateTweetRequest(BaseModel):
    content: str

async def get_current_user(request: Request):
    auth_header = request.headers.get("Authorization")
    if not auth_header or not auth_header.startswith("Bearer "):
        raise HTTPException(status_code=401, detail="Unauthorized")
    token = auth_header.split(" ")[1]
    try:
        payload = jwt.decode(token, JWT_SECRET, algorithms=["HS256"])
        return payload
    except Exception as e:
        print(f"Auth error: {e}")
        raise HTTPException(status_code=401, detail="Unauthorized")

@app.post("/api/auth/register", status_code=201)
async def register(payload: AuthRequest):
    password_hash = hashlib.sha256(payload.password.encode()).hexdigest()
    try:
        await pool.execute(
            "INSERT INTO users (username, password_hash) VALUES ($1, $2)",
            payload.username, password_hash
        )
    except asyncpg.UniqueViolationError:
        raise HTTPException(status_code=400, detail="Username already exists")

@app.post("/api/auth/login")
async def login(payload: AuthRequest):
    password_hash = hashlib.sha256(payload.password.encode()).hexdigest()
    row = await pool.fetchrow(
        "SELECT id FROM users WHERE username = $1 AND password_hash = $2",
        payload.username, password_hash
    )
    if not row:
        raise HTTPException(status_code=401, detail="Invalid credentials")
    
    token = jwt.encode(
        {"sub": str(row["id"]), "name": payload.username},
        JWT_SECRET,
        algorithm="HS256"
    )
    if isinstance(token, bytes):
        token = token.decode("utf-8")
    return {"token": token}

@app.get("/api/feed")
async def get_feed(user: dict = Depends(get_current_user)):
    rows = await pool.fetch("""
        SELECT t.id, u.username, t.content, t.created_at as "createdAt", (SELECT COUNT(*) FROM likes l WHERE l.tweet_id = t.id) as likes
        FROM tweets t
        JOIN users u ON t.user_id = u.id
        ORDER BY t.created_at DESC
        LIMIT 20
    """)
    return [dict(row) for row in rows]

@app.get("/api/tweets/{id}")
async def get_tweet(id: int, user: dict = Depends(get_current_user)):
    row = await pool.fetchrow("""
        SELECT t.id, u.username, t.content, t.created_at as "createdAt", (SELECT COUNT(*) FROM likes l WHERE l.tweet_id = t.id) as likes
        FROM tweets t
        JOIN users u ON t.user_id = u.id
        WHERE t.id = $1
    """, id)
    
    if not row:
        raise HTTPException(status_code=404, detail="Tweet not found")
    return dict(row)

@app.post("/api/tweets", status_code=201)
async def create_tweet(payload: CreateTweetRequest, user: dict = Depends(get_current_user)):
    row = await pool.fetchrow(
        "INSERT INTO tweets (user_id, content) VALUES ($1, $2) RETURNING id",
        int(user["sub"]), payload.content
    )
    return {"id": row["id"]}

@app.post("/api/tweets/{id}/like")
async def like_tweet(id: int, user: dict = Depends(get_current_user)):
    user_id = int(user["sub"])
    result = await pool.execute(
        "DELETE FROM likes WHERE user_id = $1 AND tweet_id = $2",
        user_id, id
    )
    if result == "DELETE 0":
        try:
            await pool.execute(
                "INSERT INTO likes (user_id, tweet_id) VALUES ($1, $2)",
                user_id, id
            )
        except asyncpg.ForeignKeyViolationError:
            pass # Tweet might not exist
        except asyncpg.UniqueViolationError:
            pass # Race condition
    return Response(status_code=200)
