<div align="center">

  <img src="web-app/public/logo.svg" alt="Web Framework Benchmark Logo" width="120" height="120" />

  # Web Framework Benchmark

  **The ultimate tool for comparing web framework performance across languages.**
  
  [![Rust](https://img.shields.io/badge/built_with-Rust-dca282.svg?logo=rust)](https://www.rust-lang.org/)
  [![React](https://img.shields.io/badge/frontend-React-61DAFB.svg?logo=react)](https://reactjs.org/)
  [![TypeScript](https://img.shields.io/badge/language-TypeScript-3178C6.svg?logo=typescript)](https://www.typescriptlang.org/)
  [![Docker](https://img.shields.io/badge/container-Docker-2496ED.svg?logo=docker)](https://www.docker.com/)
  [![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

  [Features](#features) • [Architecture](#architecture) • [Quick Start](#quick-start) • [Adding Benchmarks](docs/GUIDE_ADDING_BENCHMARKS.md) • [Contributing](#contributing)

</div>

<br />

<div align="center">
  <img src="assets/preview.png" alt="Dashboard Preview" width="100%" style="border-radius: 10px; box-shadow: 0 4px 8px rgba(0,0,0,0.1);" />
</div>

<br />

## 🚀 Overview

**Web Framework Benchmark (WFB)** is a comprehensive, automated benchmarking infrastructure designed to compare the throughput, latency, and resource usage of web frameworks across different programming languages.

It combines a high-performance **Rust** runner and load generator with a modern **React** dashboard to visualize results, making it easy to spot performance bottlenecks and compare implementations side-by-side.

## ✨ Features

- **📊 Multi-language Support**: Benchmarks for **C**, **C++**, **C#**, **Go**, **Java**, **JavaScript**, **Kotlin**, **Lua**, **Python**, and **Rust**.
- **🧪 Comprehensive Test Suite**:
  - **[Plaintext](docs/specs/plaintext_spec.md)**: Baseline throughput (Hello World).
  - **[JSON Analytics](docs/specs/json_aggregate_spec.md)**: Request parsing, in-memory aggregation, and response serialization.
  - **[Static Files](docs/specs/static_files_spec.md)**: Serving static binary files with correct HTTP semantics.
  - **[Database Complex](docs/specs/db_complex_spec.md)**: Realistic "Master-Detail" operation, mixing reads and writes (Interactive User Profile).
- **⚡ High-Performance Benchmarking**: Powered by `wrkr`, a custom-built Rust load generator.
- **📈 Modern Dashboard**: Interactive visualizations built with React, TypeScript, and Tailwind CSS.
- **🐳 Docker Integration**: Fully containerized environments for consistent, reproducible results.
- **🔧 Flexible Config**: YAML-based configuration for environments, languages, and test scenarios.

## 🏗 Architecture

The project is organized as a Rust workspace with a separate frontend:

1.  **wfb-runner**: The CLI tool that orchestrates Docker containers and runs benchmarks.
2.  **wfb-server**: The API server that provides data to the dashboard.
3.  **wfb-storage**: Shared library for configuration, storage logic, and data models.
4.  **wrkr**: Custom high-performance, asynchronous load generator.
5.  **web-app**: A polished React frontend to view and analyze benchmark runs.

## 🏁 Quick Start

### Prerequisites

- **Rust** (2024 edition)
- **Node.js** (18+)
- **Docker** (Running)

### 1. Build the Components

```bash
git clone https://github.com/nogcio/web-framework-benchmark.git
cd web-framework-benchmark
cargo build --release
```

### 2. Run a Benchmark

Execute the configured benchmarks using the runner.

```bash
# Run the entire suite with Run ID "1"
cargo run --release --bin wfb-runner -- run 1 --env local

# OR run a single benchmark for development/testing
cargo run --release --bin wfb-runner -- dev <benchmark_name> --env local
```

### 3. Launch the Dashboard

Start the API server and the frontend to view results.

**Terminal 1 (API Server):**
```bash
cargo run --release --bin wfb-server
```

**Terminal 2 (Frontend):**
```bash
cd web-app
npm install
npm run dev
```

Visit `http://localhost:5173` to see your results!

## 🤝 Contributing

We welcome contributions! The project is community-driven, and **anyone can add a new framework benchmark via a Pull Request**.

If you want to add your favorite framework:
1.  Read the [Adding a New Benchmark](docs/GUIDE_ADDING_BENCHMARKS.md) guide.
2.  Implement the benchmark following the specs.
3.  Submit a PR!

Whether it's adding a new framework, improving the dashboard, or fixing bugs, we appreciate your help. Please check out [CONTRIBUTING.md](CONTRIBUTING.md) for general guidelines.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
