<div align="center">

  <img src="web-app/public/logo.svg" alt="Web Framework Benchmark Logo" width="120" height="120" />

  # Web Framework Benchmark

  **The ultimate tool for comparing web framework performance across languages.**
  
  [![Rust](https://img.shields.io/badge/built_with-Rust-dca282.svg?logo=rust)](https://www.rust-lang.org/)
  [![React](https://img.shields.io/badge/frontend-React-61DAFB.svg?logo=react)](https://reactjs.org/)
  [![TypeScript](https://img.shields.io/badge/language-TypeScript-3178C6.svg?logo=typescript)](https://www.typescriptlang.org/)
  [![Docker](https://img.shields.io/badge/container-Docker-2496ED.svg?logo=docker)](https://www.docker.com/)
  [![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

  [Features](#features) • [Architecture](#architecture) • [Quick Start](#quick-start) • [Contributing](#contributing)

</div>

<br />

<div align="center">
  <img src="assets/preview.png" alt="Dashboard Preview" width="100%" style="border-radius: 10px; box-shadow: 0 4px 8px rgba(0,0,0,0.1);" />
</div>

<br />

## 🚀 Overview

**Web Framework Benchmark (WFB)** is a comprehensive, automated benchmarking infrastructure designed to compare the throughput, latency, and resource usage of web frameworks across different programming languages (Rust, C#, Go, Python, etc.).

It combines a high-performance **Rust** runner with a modern **React** dashboard to visualize results, making it easy to spot performance bottlenecks and compare implementations side-by-side.

## ✨ Features

- **📊 Multi-language Support**: Extensible architecture to benchmark frameworks in any language (currently focused on C#/.NET, with more coming).
- **🧪 Comprehensive Test Suite**:
  - **Hello World**: Baseline throughput.
  - **JSON Serialization**: CPU-bound processing.
  - **Database Operations**: Single read, paginated reads, and writes (PostgreSQL, MySQL, MariaDB, MSSQL, MongoDB).
  - **Static Files**: Serving assets of various sizes.
  - **Real World Scenario**: A "Tweet Service" API simulation with Auth, DB relationships, and more complex logic.
- **⚡ Automated Benchmarking**: Powered by `wrk` for high-performance load generation.
- **📈 Modern Dashboard**: Interactive visualizations built with React, TypeScript, and Tailwind CSS.
- **🐳 Docker Integration**: Fully containerized environments for consistent, reproducible results.
- **🔧 Flexible Config**: YAML-based configuration for environments, languages, and test scenarios.

## 🏗 Architecture

The project consists of two main parts:

1.  **Core Engine (`src/`)**: A Rust application that orchestrates Docker containers, runs `wrk` benchmarks, collects metrics, and serves the API.
2.  **Web Dashboard (`web-app/`)**: A polished frontend to view and analyze benchmark runs.

## 🏁 Quick Start

### Prerequisites

- **Rust** (2024 edition+)
- **Node.js** (18+)
- **Docker** (Running)

### 1. Build the CLI

```bash
git clone https://github.com/nogcio/web-framework-benchmark.git
cd web-framework-benchmark
cargo build --release
```

### 2. Run a Benchmark

Execute the configured benchmarks. Results are saved to `data/`.

```bash
# Run with ID "1" in local environment
cargo run --release -- run 1 --environment local
```

### 3. Launch the Dashboard

Start the API server and the frontend to view results.

**Terminal 1 (API Server):**
```bash
cargo run --release -- serve
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
1.  Read the [Adding a New Framework](ADDING_FRAMEWORK.md) guide.
2.  Implement the benchmark following the specs.
3.  Submit a PR!

Whether it's adding a new framework, improving the dashboard, or fixing bugs, we appreciate your help. Please check out [CONTRIBUTING.md](CONTRIBUTING.md) for general guidelines.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
