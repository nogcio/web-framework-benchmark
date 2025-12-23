# Adding a New Framework Benchmark

This guide provides a comprehensive, step-by-step process for adding a new framework benchmark to the Web Framework Benchmark project. It covers directory structure, configuration, implementation requirements, and testing scenarios.

## Table of Contents

1.  [Prerequisites](#1-prerequisites)
2.  [Step 1: Directory Structure](#2-step-1-directory-structure)
3.  [Step 2: Configuration](#3-step-2-configuration)
4.  [Step 3: Dockerfile](#4-step-3-dockerfile)
5.  [Step 4: Implementation Requirements](#5-step-4-implementation-requirements)
6.  [Step 5: Test Scenarios](#6-step-5-test-scenarios)
    *   [Hello World](#61-hello-world)
    *   [JSON Serialization](#62-json-serialization)
    *   [Database Tests](#63-database-tests)
    *   [Static Files](#64-static-files)
    *   [Tweet Service (Real World)](#65-tweet-service-real-world)
7.  [Step 6: Database Schema](#7-step-6-database-schema)
8.  [Step 7: Running and Verifying](#8-step-7-running-and-verifying)

---

## 1. Prerequisites

Before you begin, ensure you have the following installed:
*   **Docker**: The benchmark runs entirely within Docker containers.
*   **Rust**: Required to build and run the benchmark runner (`wfb`).
*   **Git**: To clone the repository and manage your changes.

---

## 2. Step 1: Directory Structure

Benchmarks are organized by language and framework. Create a new directory for your framework under `benchmarks/<language>/<framework>`.

**Naming Convention:** Use lowercase, kebab-case for directory names (e.g., `aspnetcore-efcore`, `fastapi-pg`).

**Example:** Adding a `FastAPI` benchmark for Python.

```
benchmarks/python/fastapi/
├── Dockerfile
├── requirements.txt (or pyproject.toml, etc.)
└── src/
    └── main.py
```

---

## 3. Step 2: Configuration

You must register your benchmark in the configuration files located in the `config/` directory.

### 3.1. Language (`config/languages.yaml`)

If the programming language is not already present, add it to `config/languages.yaml`.

```yaml
- name: Python
  url: https://www.python.org
```

### 3.2. Framework (`config/frameworks.yaml`)

Register the framework in `config/frameworks.yaml`. This metadata is used for reporting.

```yaml
- name: fastapi
  language: Python
  url: https://fastapi.tiangolo.com/
```

### 3.3. Benchmark (`config/benchmarks.yaml`)

Define the specific benchmark configuration in `config/benchmarks.yaml`. This links your code, the tests it supports, and its metadata.

**Minimal Example (No Database):**

```yaml
- name: fastapi
  language: Python
  language_version: "3.11"
  framework: fastapi
  framework_version: "0.100.0"
  tests:
    - hello_world
    - json
  tags:
    platform: python
    python: "3.11"
  path: benchmarks/python/fastapi
```

**Database Example:**

If your benchmark uses a database, specify the `database` field (e.g., `postgres`, `mysql`, `mssql`, `mongodb`) and include the relevant database tests.

```yaml
- name: fastapi-pg
  language: Python
  language_version: "3.11"
  framework: fastapi
  framework_version: "0.100.0"
  tests:
    - db_read_one
    - db_read_paging
    - db_write
  tags:
    platform: python
    python: "3.11"
    orm: sqlalchemy
  path: benchmarks/python/fastapi-pg
  database: postgres
```

---

## 4. Step 3: Dockerfile

You must provide a `Dockerfile` in your benchmark directory that builds and runs your application.

*   **Base Image**: Use an official and appropriate base image (e.g., `python:3.11-slim`, `golang:1.21-alpine`).
*   **Port**: The application **MUST** listen on port **8000**. You may use the `PORT` environment variable if provided, but defaulting to 8000 is required.
*   **Command**: The container must start the web server automatically.
*   **Production Ready**: Ensure the application runs in "production" mode (e.g., `NODE_ENV=production`, `ASPNETCORE_ENVIRONMENT=Production`).

**Example (Python):**
```dockerfile
FROM python:3.11-slim
WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
COPY src/ .
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
```

---

## 5. Step 4: Implementation Requirements

Your application must implement the endpoints required for the tests you enabled in `config/benchmarks.yaml`.

### General Requirements

*   **Port**: Listen on port **8000**.
*   **Headers**:
    *   **X-Request-ID**: If the request contains an `X-Request-ID` header, the response **MUST** include the same header with the same value. This is used to verify that responses match requests.
*   **Database Connection**:
    *   Use environment variables for connection details:
        *   `DB_HOST` (default: `db` or `localhost`)
        *   `DB_PORT` (default: `5432`, `3306`, or `1433`)
        *   `DB_NAME` (default: `benchmark`)
        *   `DB_USER` (default: `benchmark`)
        *   `DB_PASSWORD` (default: `benchmark`)

---

## 6. Step 5: Test Scenarios

Implement the endpoints corresponding to the tests listed in your `config/benchmarks.yaml`.

### 6.1. Hello World
*   **Test Name**: `hello_world`
*   **Endpoint**: `GET /`
*   **Response Body**: `Hello, World!` (exact string match)
*   **Content-Type**: `text/plain` (optional but recommended)
*   **Status Code**: 200 OK

### 6.2. JSON Serialization
*   **Test Name**: `json`
*   **Endpoint**: `POST /json/{from}/{to}`
*   **Request Body**: A large JSON object (structure defined in `scripts/wrk_json.lua`).
*   **Logic**:
    1.  Deserialize the request body.
    2.  Find all occurrences of the field `servlet-name` where the value equals the `{from}` path parameter.
    3.  Replace those values with the `{to}` path parameter.
    4.  Serialize the modified object back to JSON.
*   **Response Body**: The modified JSON object.
*   **Status Code**: 200 OK

### 6.3. Database Tests

These tests require a database connection.

#### Database Read One (`db_read_one`)
*   **Endpoint**: `GET /db/read/one?id={id}`
*   **Query Parameter**: `id` (integer, 1-1000)
*   **Logic**: Fetch a single row from the `hello_world` table where `id` matches the parameter.
*   **Response Body**: JSON representation of the row.
    ```json
    {
      "id": 123,
      "name": "name_123",
      "created_at": "2023-01-01T00:00:00Z",
      "updated_at": "2023-01-01T00:00:00Z"
    }
    ```
*   **Status Code**: 200 OK

#### Database Read Paging (`db_read_paging`)
*   **Endpoint**: `GET /db/read/many?offset={offset}&limit={limit}`
*   **Query Parameters**:
    *   `offset` (integer)
    *   `limit` (integer, default 50)
*   **Logic**: Fetch rows from the `hello_world` table ordered by `id`, using the specified limit and offset.
*   **Response Body**: JSON array of rows.
*   **Status Code**: 200 OK

#### Database Write (`db_write`)
*   **Endpoint**: `POST /db/write/insert`
*   **Request Body**: JSON `{"name": "..."}`
*   **Logic**:
    1.  Insert a new row into the `hello_world` table with the provided `name`.
    2.  Set `created_at` and `updated_at` to the current timestamp.
    3.  Return the inserted row.
*   **Response Body**: JSON representation of the inserted row (including the generated `id`).
*   **Status Code**: 200 OK

### 6.4. Static Files
*   **Test Name**: `static_files_small`, `static_files_medium`, `static_files_large`
*   **Endpoint**: `GET /files/{filename}`
*   **Logic**: Serve the requested file from the `benchmarks_data` directory (mounted at runtime).
*   **Security**: Ensure no directory traversal attacks (e.g., `../`).
*   **Status Code**: 200 OK

### 6.5. Tweet Service (Real World)
*   **Test Name**: `tweet_service`
*   **Description**: A complex scenario simulating a social network API. Covers Routing, Auth (JWT), DB (Read/Write), and JSON.

#### Entities
*   **Users**: `id`, `username` (unique), `password_hash`
*   **Tweets**: `id`, `user_id`, `content`, `created_at`
*   **Likes**: `user_id`, `tweet_id` (unique pair)

#### Endpoints (Auth Required via JWT Bearer)

All endpoints **MUST** handle the `X-Request-ID` header: if present in the request, it must be echoed back in the response.

1.  **Register**
    *   `POST /api/auth/register`
    *   Body: `{"username": "...", "password": "..."}`
    *   Logic: Create user. **MUST** use **SHA256** for password hashing.
    *   Response: `201 Created`

2.  **Login**
    *   `POST /api/auth/login`
    *   Body: `{"username": "...", "password": "..."}`
    *   Logic: Verify credentials (SHA256). Return JWT with `sub` (user_id) and `name` (username).
    *   Response: `200 OK` `{"token": "..."}`

3.  **Feed**
    *   `GET /api/feed`
    *   Logic: Return 20 most recent tweets from *any* user.
    *   Response: `200 OK` `[{"id": 1, "username": "...", "content": "...", "likes": 5}, ...]`

4.  **Get Tweet**
    *   `GET /api/tweets/{id}`
    *   Logic: Return tweet details.
    *   Response: `200 OK`

5.  **Create Tweet**
    *   `POST /api/tweets`
    *   Body: `{"content": "..."}`
    *   Logic: Create tweet for current user.
    *   Response: `201 Created`

6.  **Like Tweet**
    *   `POST /api/tweets/{id}/like`
    *   Logic: Toggle like (Add if missing, Remove if exists).
    *   Response: `200 OK`

---

## 7. Step 6: Database Schema

The benchmark runner handles database initialization. However, your code should expect the following schemas.

### Standard Schema (`hello_world`)

```sql
CREATE TABLE hello_world (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);
```

### Tweet Service Schema

```sql
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(128) NOT NULL UNIQUE,
    password_hash VARCHAR(64) NOT NULL
);

CREATE TABLE tweets (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL REFERENCES users(id),
    content VARCHAR(256) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE likes (
    user_id INT NOT NULL REFERENCES users(id),
    tweet_id INT NOT NULL REFERENCES tweets(id),
    PRIMARY KEY (user_id, tweet_id)
);
CREATE INDEX idx_likes_tweet_id ON likes(tweet_id);
```

### MongoDB Schema

For MongoDB, the collections should follow this structure. Note that `_id` is an `ObjectId`.

**Collection: `hello_world`**
```json
{
  "_id": ObjectId("..."),
  "name": "name_1",
  "created_at": ISODate("..."),
  "updated_at": ISODate("...")
}
```

**Collection: `users`**
```json
{
  "_id": ObjectId("..."),
  "username": "user_1",
  "password_hash": "..."
}
```
*Index: `username` (unique)*

**Collection: `tweets`**
```json
{
  "_id": ObjectId("..."),
  "user_id": ObjectId("..."), // Reference to users._id
  "content": "...",
  "created_at": ISODate("...")
}
```
*Index: `user_id`*

**Collection: `likes`**
```json
{
  "_id": ObjectId("..."),
  "user_id": ObjectId("..."), // Reference to users._id
  "tweet_id": ObjectId("...") // Reference to tweets._id
}
```
*Index: `user_id`, `tweet_id` (unique compound)*

---

## 8. Step 7: Running and Verifying

Use the benchmark runner to verify your implementation.

1.  **Build the runner**:
    ```bash
    cargo build --release
    ```

2.  **Run your benchmark**:
    Use the `--filter` argument to run only your new benchmark. You also need to provide a run ID (e.g., 1).
    ```bash
    cargo run --release -- run 1 --filter <your_benchmark_name>
    ```

3.  **Check for errors**:
    If the benchmark fails, check the logs. The runner will output the container logs if a test fails.

4.  **Verify Results**:
    Ensure that `requests_per_sec` is reasonable and `errors` is 0.

---

## 9. Manual Verification (Debugging)

Sometimes it is useful to run the benchmark manually to debug issues or verify behavior without the full runner.

### 1. Create a Docker Network
Create a dedicated network so containers can communicate.
```bash
docker network create wfb-network
```

### 2. Start the Database
Build and run the database container (e.g., Postgres).
```bash
# Build DB image (from project root)
docker build -t wfb-db-postgres benchmarks_db/pg

# Run DB container
docker run -d --name db --network wfb-network \
    -e POSTGRES_USER=benchmark \
    -e POSTGRES_PASSWORD=benchmark \
    -e POSTGRES_DB=benchmark \
    wfb-db-postgres
```

### 3. Build and Run Your Benchmark
```bash
# Build your benchmark image
cd benchmarks/<language>/<framework>
docker build -t wfb-app .

# Run application container
# Note: We link to the 'db' container using the hostname 'db'
docker run -d --name app --network wfb-network -p 8000:8000 \
    -e DB_HOST=db \
    -e DB_PORT=5432 \
    -e DB_USER=benchmark \
    -e DB_PASSWORD=benchmark \
    -e DB_NAME=benchmark \
    wfb-app
```

### 4. Run wrk
Install `wrk` (e.g., `brew install wrk` or `apt install wrk`) and run it against your local port.

**Example: JSON Test**
```bash
# Run from project root
wrk -t2 -c100 -d10s -s scripts/wrk_json.lua http://localhost:8000/json/1/2
```

**Example: Database Read One**
```bash
# For SQL databases
wrk -t2 -c100 -d10s -s scripts/wrk_db_read_one.lua "http://localhost:8000/db/read/one?id=1"

# For MongoDB (uses different ID format)
wrk -t2 -c100 -d10s -s scripts/wrk_db_read_one_mongo.lua "http://localhost:8000/db/read/one?id=000000000000000000000001"
```

**Example: Tweet Service**
```bash
# For SQL databases
wrk -t2 -c100 -d10s -s scripts/wrk_tweet_service.lua http://localhost:8000/api

# For MongoDB
wrk -t2 -c100 -d10s -s scripts/wrk_tweet_service_mongo.lua http://localhost:8000/api
```

### 5. Cleanup
```bash
docker rm -f app db
docker network rm wfb-network
```
