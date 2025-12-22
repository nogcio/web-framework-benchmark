# Web Framework Benchmark (WFB)

A comprehensive benchmarking tool for comparing the performance of web frameworks and HTTP services across different programming languages. This project provides automated benchmarking infrastructure, result visualization, and extensible framework support.

## Features

- **Multi-language Support**: Benchmark frameworks written in different languages (currently C#, extensible to others)
- **Comprehensive Test Suite**: Includes tests for:
  - Hello World responses
  - JSON serialization/deserialization
  - Database read operations (single and paginated)
  - Database write operations
  - Static file serving
- **Automated Benchmarking**: Uses `wrk` for high-performance HTTP load testing
- **Result Visualization**: Modern web dashboard built with React, TypeScript, and Tailwind CSS
- **API-Driven Architecture**: Rust-based backend serves benchmark results via a REST API
- **Result Storage**: Local filesystem storage of benchmark results in YAML format
- **Database Integration**: Support for PostgreSQL, MySQL, and MSSQL test databases
- **Docker Support**: Containerized environments for consistent benchmarking
- **Local and Remote Environments**: Support for both local development and remote deployment

## Architecture

The project consists of several key components:

- **Core Engine (`src/`)**: A Rust application that serves two purposes:
  - **Runner**: Executes benchmarks using `wrk`, manages Docker containers, and collects metrics.
  - **API Server**: Provides a REST API to serve benchmark configurations and results to the frontend.
- **Web Dashboard (`web-app/`)**: A React application (Vite + Tailwind + TanStack Query) that consumes the Core API to visualize benchmark results.
- **Framework Implementations (`benchmarks/`)**: Benchmark implementations for various frameworks (currently focused on C#/.NET).
- **Databases (`benchmarks_db/`)**: Docker configurations and initialization scripts for benchmark databases (PostgreSQL, MySQL, MSSQL).
- **Configuration (`config/`)**: YAML-based configuration for languages, frameworks, benchmarks, and environments.
- **Data Storage (`data/`)**: Benchmark run results are stored as YAML files in the filesystem.
- **Scripts (`scripts/`)**: Lua scripts used by `wrk` to generate load for different test scenarios.

## Quick Start

### Prerequisites

- Rust (2024 edition or later)
- Node.js (18+)
- Docker

### 1. Build the Rust CLI

```bash
# Clone the repository
git clone https://github.com/nogcio/web-framework-benchmark.git
cd web-framework-benchmark

# Build the CLI tool
cargo build --release
```

### 2. Run Benchmarks

To run benchmarks, you use the `run` command. This will execute the configured benchmarks and save the results to the `data/` directory.

```bash
# Run all benchmarks with run ID "1" in the local environment
cargo run --release -- run 1 --environment local
```

### 3. Start the API Server

The API server provides access to the benchmark results.

```bash
# Start the server on localhost:8080
cargo run --release -- serve
```

### 4. Start the Web Dashboard

In a new terminal, start the frontend development server. It is configured to proxy API requests to the Rust server running on port 8080.

```bash
cd web-app
npm install
npm run dev
```

Open http://localhost:5173 to view the dashboard.

## Usage

### CLI Commands

The `wfb` tool has two main commands: `run` and `serve`.

#### Run Command
Execute configured benchmarks and store results.

```bash
cargo run --release -- run <id> [--environment <type>]
```

- `id`: Unique identifier for this benchmark run (e.g., `1`, `2023-10-27`).
- `environment`: The environment configuration to use (default: `local`). Defined in `config/environments/`.

#### Serve Command
Start the REST API server to expose benchmark data.

```bash
cargo run --release -- serve [--host <host>] [--port <port>]
```

- `host`: Host to bind to (default: `127.0.0.1`).
- `port`: Port to listen on (default: `8080`).

## Project Structure

```
├── benchmarks/         # Framework implementations (e.g., csharp/aspnetcore)
├── benchmarks_data/    # Static files used during benchmarking
├── benchmarks_db/      # Database configurations (PostgreSQL, MySQL, MSSQL)
├── config/             # Configuration files (benchmarks.yaml, frameworks.yaml, etc.)
├── data/               # Stored benchmark results
├── scripts/            # Lua scripts for wrk
├── src/                # Rust source code for the CLI and API server
└── web-app/            # React frontend application
```
