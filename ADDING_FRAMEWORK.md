# Adding a New Framework Benchmark

This document outlines the requirements and steps to add a new framework benchmark to the project.

## 1. Directory Structure

Create a new directory for your framework under `benchmarks/<language>/<framework>`.
For example, if you are adding a benchmark for `FastAPI` in Python:

```
benchmarks/python/fastapi/
├── Dockerfile
├── requirements.txt (or pyproject.toml, etc.)
└── src/
    └── main.py
```

## 2. Configuration

Register your new framework in `config/languages.yaml`.

```yaml
- name: Python
  url: https://www.python.org
  frameworks:
    - name: fastapi
      path: benchmarks/python/fastapi
      url: https://fastapi.tiangolo.com/
      tags:
        python: "3.11"
        platform: python
```

## 3. Dockerfile

You must provide a `Dockerfile` that builds and runs your application.
*   **Base Image**: Use an official and appropriate base image (e.g., `python:3.11-slim`, `golang:1.21-alpine`).
*   **Port**: The application must listen on port **8000** (or use the `PORT` env var).
*   **Command**: The container must start the web server automatically.

Example (Python):
```dockerfile
FROM python:3.11-slim
WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
COPY src/ .
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
```

## 4. Environment Variables

Your application must respect the following environment variables:

| Variable | Default | Description |
| :--- | :--- | :--- |
| `PORT` | `8000` | The port to listen on. |
| `DB_HOST` | `db` | PostgreSQL database host. |
| `DB_PORT` | `5432` | PostgreSQL database port. |
| `DB_USER` | `benchmark` | Database user. |
| `DB_PASSWORD` | `benchmark` | Database password. |
| `DB_NAME` | `benchmark` | Database name. |
| `DATA_DIR` | `benchmarks_data` | Directory containing static files to serve. |

## 5. Required Endpoints

Your application must implement the following endpoints. All JSON responses should have `Content-Type: application/json`.

**Important**: All endpoints must echo the `x-request-id` request header in the response if it is present. This is used by the verification scripts to match requests and responses.

### 5.1. Root / Hello World
*   **Path**: `GET /`
*   **Response**: Plain text "Hello, World!"
*   **Status**: 200 OK

### 5.2. Health Check
*   **Path**: `GET /health`
*   **Logic**: Check if the database connection is active.
*   **Response**: "OK" (or similar) if healthy.
*   **Status**: 200 OK if healthy, 503 Service Unavailable if DB is down.

### 5.3. Info
*   **Path**: `GET /info`
*   **Response**: A comma-separated string of capabilities/versions.
*   **Example**: `1.21,hello_world,json,db_read_one,db_read_paging,db_write,static_files`
*   **Status**: 200 OK

### 5.4. JSON Processing
*   **Path**: `POST /json/{from}/{to}`
*   **Logic**:
    1.  Parse the request body as JSON.
    2.  Traverse the JSON structure.
    3.  Find all objects where the key is `servlet-name` and the value equals `{from}`.
    4.  Replace the value with `{to}`.
    5.  Return the modified JSON.
*   **Status**: 200 OK

### 5.5. Database: Read One
*   **Path**: `GET /db/read/one`
*   **Query Param**: `id` (integer)
*   **Logic**: Fetch a single row from the `hello_world` table by `id`.
*   **Response JSON**:
    ```json
    {
      "id": 1,
      "name": "...",
      "created_at": "...",
      "updated_at": "..."
    }
    ```
*   **Status**: 200 OK, or 404 if not found.

### 5.6. Database: Read Many (Paging)
*   **Path**: `GET /db/read/many`
*   **Query Params**:
    *   `offset` (integer, required)
    *   `limit` (integer, optional, default 50)
*   **Logic**: Fetch rows from `hello_world` ordered by `id` with limit and offset.
*   **Response JSON**: Array of objects (same format as Read One).
*   **Status**: 200 OK

### 5.7. Database: Write (Insert)
*   **Path**: `POST /db/write/insert`
*   **Input**: JSON `{"name": "..."}` OR Query Param `name`.
*   **Logic**: Insert a new row into `hello_world` with the provided name and current timestamp for `created_at` and `updated_at`.
*   **Response JSON**: The created object (including the generated ID).
*   **Status**: 200 OK

### 5.8. Static Files
*   **Path**: `GET /files/{filename}`
*   **Logic**: Serve the file named `{filename}` from the `DATA_DIR`.
*   **Security**: Prevent directory traversal (e.g., `..`).
*   **Files to support**: `15kb.bin`, `1mb.bin`, `10mb.bin`.
*   **Status**: 200 OK

## 6. Database Schema and Data

The PostgreSQL database has the following schema:

```sql
CREATE TABLE hello_world (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);
```

The table is pre-populated with 1000 rows where `id` ranges from 1 to 1000, and `name` is `name_<id>` (e.g., `name_1`, `name_2`, ..., `name_1000`).
The verification scripts rely on this data pattern.


## 7. Verification

The project uses `wrk` with Lua scripts (located in `scripts/`) to verify and benchmark your implementation.
Ensure your implementation passes the logic checks in these scripts:
*   `wrk_hello.lua`
*   `wrk_json.lua`
*   `wrk_db_read_one.lua`
*   `wrk_db_read_paging.lua`
*   `wrk_db_write.lua`
*   `wrk_static_files.lua`
