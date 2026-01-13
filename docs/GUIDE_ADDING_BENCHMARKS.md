# How to Add a New Benchmark

This guide provides a step-by-step walkthrough for adding a new language or framework to the Web Framework Benchmark (WFB).

## 1. Directory Structure

All benchmarks live in the `benchmarks/` directory, organized by language and then by framework/implementation name.

**Pattern:** `benchmarks/<language_slug>/<framework_slug>`

**Example:**
If you are adding a benchmark for **Axum** in **Rust**:
1.  Navigate to `benchmarks/rust/`.
2.  Create a folder named `axum`.

## 2. Benchmark Implementation

### Dockerfile Requirements

Your benchmark folder must contain a `Dockerfile`.

*   **Port**: The application **MUST** listen on port **8080**.
*   **Healthcheck**: You **MUST** define a `HEALTHCHECK` instruction.
*   **Base Image**: Use specific versions for reproducibility.
*   **Production**: Ensure the application runs in production mode.

**Example Dockerfile (Rust/Axum):**
```dockerfile
# Build Stage
FROM rust:1.92 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Runtime Stage
FROM debian:bookworm-slim
# Install curl for healthcheck
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/wfb-rust-axum .

# If serving static files
# Note: The runner automatically provides benchmarks_data in the build context.
# If building manually with `docker build`, you must copy this folder to the context root or comment this line.
COPY benchmarks_data /app/benchmarks_data 

ENV PORT=8080

# MANDATORY: Healthcheck
HEALTHCHECK --interval=5s --timeout=3s --retries=3 \
  CMD curl --fail http://localhost:8080/health || exit 1

CMD ["./wfb-rust-axum"]
```

### Environment Variables

The runner injects the following environment variables into your container. Your application **MUST** support these variables, especially for database connections.

| Variable | Description | Default Value |
| :--- | :--- | :--- |
| `PORT` | The port the application listens on. | `8080` |
| `DATA_DIR` | Directory containing benchmark data files. | `benchmarks_data` |
| `DB_HOST` | Database hostname. | *Dynamic* |
| `DB_PORT` | Database port. | *Dynamic* |
| `DB_USER` | Database username. | `user` |
| `DB_PASSWORD` | Database password. | `password` (or `Benchmark!12345` for MSSQL) |
| `DB_NAME` | Database name. | `hello_world` |
| `DB_KIND` | Type of database (e.g., `postgres`, `mysql`). | *Depends on benchmark config* |
| `DB_POOL_SIZE` | Recommended database connection pool size. | `256` |

### Implementing Test Cases

You need to implement endpoints that correspond to the test specifications in `docs/specs/`.

#### 0. Healthcheck (`/health`)
*   **Requirement**: MANDATORY
*   **Method**: `GET`
*   **Response**: `200 OK`
*   **Purpose**: Used by Docker and the runner to verify the service is ready.
*   **Database**: If the benchmark uses a database, this endpoint **MUST** verify the database connection is active before returning `200 OK`.

#### 1. Plaintext (`/plaintext`)
*   **Spec**: [docs/specs/plaintext_spec.md](specs/plaintext_spec.md)
*   **Method**: `GET`
*   **Response**: `200 OK`, `Content-Type: text/plain`, Body: "Hello, World!"

#### 2. JSON Analytics (`/json`)
*   **Spec**: [docs/specs/json_aggregate_spec.md](specs/json_aggregate_spec.md)
*   **Method**: `GET`
*   **Logic**: Parse JSON body, aggregation logic, return JSON.

#### 3. Static Files (`/static/*`)
*   **Spec**: [docs/specs/static_files_spec.md](specs/static_files_spec.md)
*   **Logic**: Serve files from a directory.
*   *Requirement*: Must support `Range` headers and proper caching.

#### 4. Database Complex (`/db/user-profile/:email`)
*   **Spec**: [docs/specs/db_complex_spec.md](specs/db_complex_spec.md)
*   **Method**: `GET`
*   **Logic**: Fetch complex user profile from 4 related tables, calculate stats, return JSON.

#### 5. gRPC Aggregate (`/AnalyticsService/AggregateOrders`)
*   **Spec**: [docs/specs/grpc_aggregate_spec.md](specs/grpc_aggregate_spec.md)
*   **Method**: `gRPC Client Streaming` (POST over HTTP/2)
*   **Logic**: Streaming Order messages, aggregation logic (same as JSON), return AggregateResult.
*   **Note**: Requires implementing the protobuf service definition.

## 3. Configuration

Register your benchmark in `config/`.

### A. `config/languages.yaml` (If new)
```yaml
---
type: language
name: Rust
url: https://www.rust-lang.org
color: "#DEA584"
```

### B. `config/frameworks.yaml`
```yaml
---
type: framework
name: axum          # Framework slug
language: Rust
url: https://github.com/tokio-rs/axum
```

### C. `config/benchmarks/<language>.yaml`
```yaml
---
type: benchmark
name: axum                  # Unique identifier
language: Rust
language_version: "1.92"
framework: axum
framework_version: "0.8.8"
tests:
  - plain_text
  - static_files
  - json_aggregate
tags:
  type: micro-framework
  runtime: native
  arch: async
path: benchmarks/rust/axum  # Path to Dockerfile
```

## 4. Verification

Before running a full benchmark, you **MUST** verify that your implementation satisfies the requirements.

### 1. Run Verification
The `verify` command starts your container and checks endpoints against the specs.

```bash
# Verify the specific benchmark defined in config/benchmarks/<language>.yaml
cargo run --release --bin wfb-runner -- verify --benchmark axum --env local
```

**Check the output:**
- "Healthcheck: PASSED"
- "Plaintext: PASSED"
- "JSON: PASSED"

If verification fails, check the runner logs and your container logs (`docker logs <container_id>`).

### 2. Run Benchmark
Once verification passes, utilize the `dev` command to run the full benchmark load test for just your new framework.

```bash
cargo run --release --bin wfb-runner -- dev axum --env local
```

> **Note:** The `run` command (e.g. `run <id>`) executes the entire suite of all configured benchmarks. Use `dev` for single-framework testing.

### 3. Debugging
*   **Manual Run**: `docker run -p 8080:8080 <image_id>`
*   **Test Health**: `curl -v http://localhost:8080/health`

## 5. Submitting

1.  Commit changes.
2.  Push to fork.
3.  Open Pull Request: "Add Rust/Axum benchmark".
