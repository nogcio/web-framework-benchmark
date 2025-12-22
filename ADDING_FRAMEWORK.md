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

You need to register your benchmark in the configuration files located in the `config/` directory.

### 2.1. Language (`config/languages.yaml`)

If the language is not already present, add it to `config/languages.yaml`:

```yaml
- name: Python
  url: https://www.python.org
```

### 2.2. Framework (`config/frameworks.yaml`)

Register the framework in `config/frameworks.yaml`:

```yaml
- name: fastapi
  language: Python
  url: https://fastapi.tiangolo.com/
```

### 2.3. Benchmark (`config/benchmarks.yaml`)

Define the benchmark configuration in `config/benchmarks.yaml`. This links the code, tests, and metadata.

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

If your benchmark uses a database, specify the `database` field (e.g., `postgres`, `mysql`, `mssql`) and include the relevant database tests:

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

## 3. Dockerfile

You must provide a `Dockerfile` that builds and runs your application.

*   **Base Image**: Use an official and appropriate base image (e.g., `python:3.11-slim`, `golang:1.21-alpine`).
*   **Port**: The application must listen on port **8000** (or use the `PORT` env var if passed, but 8000 is the standard expectation).
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

## 4. Implementation Requirements

Your application must implement the endpoints required for the tests you enabled in `config/benchmarks.yaml`.

### General Requirements

*   **Port**: Listen on port **8000** (or the `PORT` environment variable).
*   **Headers**:
    *   **X-Request-ID**: If the request contains an `X-Request-ID` header, the response **MUST** include the same header with the same value. This is used to verify that responses match requests.

### Test Scenarios

#### 1. Hello World (`hello_world`)
*   **Endpoint**: `GET /`
*   **Response Body**: `Hello, World!` (exact string match)
*   **Content-Type**: `text/plain` (optional but recommended)
*   **Status Code**: 200 OK

#### 2. JSON Serialization (`json`)
*   **Endpoint**: `POST /json/{from}/{to}`
*   **Request Body**: A large JSON object (see `scripts/wrk_json.lua` for structure).
*   **Logic**:
    1.  Deserialize the request body.
    2.  Find all occurrences of `servlet-name` equal to the `{from}` path parameter.
    3.  Replace them with the `{to}` path parameter.
    4.  Serialize the modified object back to JSON.
*   **Response Body**: The modified JSON object.
*   **Status Code**: 200 OK

#### 3. Database Read One (`db_read_one`)
*   **Endpoint**: `GET /db/read/one?id={id}`
*   **Query Parameter**: `id` (integer, 1-1000)
*   **Logic**: Fetch a single row from the `hello_world` table where `id` matches the parameter.
*   **Response Body**: JSON representation of the row.
    ```json
    {
      "id": 123,
      "name": "...",
      "created_at": "...",
      "updated_at": "..."
    }
    ```
*   **Status Code**: 200 OK

#### 4. Database Read Paging (`db_read_paging`)
*   **Endpoint**: `GET /db/read/many?offset={offset}&limit={limit}`
*   **Query Parameters**:
    *   `offset` (integer)
    *   `limit` (integer, default 50)
*   **Logic**: Fetch rows from the `hello_world` table ordered by `id`, using the specified limit and offset.
*   **Response Body**: JSON array of rows.
*   **Status Code**: 200 OK

#### 5. Database Write (`db_write`)
*   **Endpoint**: `POST /db/write/insert`
*   **Request Body**: JSON `{"name": "..."}`
*   **Logic**:
    1.  Insert a new row into the `hello_world` table with the provided `name`.
    2.  Set `created_at` and `updated_at` to the current timestamp.
    3.  Return the inserted row.
*   **Response Body**: JSON representation of the inserted row (including the generated `id`).
*   **Status Code**: 200 OK

#### 6. Static Files (`static_files_*`)
*   **Endpoint**: `GET /files/{filename}`
*   **Logic**: Serve the requested file from the `benchmarks_data` directory (mounted at runtime).
*   **Security**: Ensure no directory traversal attacks (e.g., `../`).
*   **Status Code**: 200 OK

## 5. Database Schema

The database (PostgreSQL, MySQL, MSSQL) is initialized with a `hello_world` table:

```sql
CREATE TABLE hello_world (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);
```

(Syntax varies slightly by database dialect).
