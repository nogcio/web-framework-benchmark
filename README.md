<div align="center">

  <img src="assets/logo.svg" alt="Web Framework Benchmark Logo" width="120" />

  # Web Framework Benchmark

  **The ultimate tool for comparing web framework performance across languages.**
  
  [![Rust](https://img.shields.io/badge/built_with-Rust-dca282.svg?logo=rust)](https://www.rust-lang.org/)
  [![Docker](https://img.shields.io/badge/container-Docker-2496ED.svg?logo=docker)](https://www.docker.com/)
  [![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

  [Features](#features) • [Architecture](#architecture) • [Quick Start](#quick-start) • [Adding Benchmarks](docs/GUIDE_ADDING_BENCHMARKS.md) • [Contributing](#contributing)

  <br />

  <img src="assets/preview.png" alt="Web Framework Benchmark Preview" width="100%" />

</div>

<br />

## 🚀 Philosophy: Benchmarking Reality

Most web benchmarks focus on synthetic "Hello World" cases that measure raw socket performance but ignore application logic. **Web Framework Benchmark (WFB)** takes a different approach.

We measure how frameworks handle **real-world production scenarios**, prioritizing application complexity, strict correctness, and modern protocol comparisons over simple echo tests.

## 🏆 Key Differentiators

### 1. 🧠 Heavy Business Logic
We don't just dump bytes to a socket.
- **JSON Analytics**: Simulates a microservice analyzing e-commerce orders. It tests parsing efficiency, in-memory aggregation, and allocation-heavy workloads.
- **Database Complex**: A full "User Profile" endpoint mixing reads, writes, and parallel queries to build complex nested responses.

### 2. ⚔️ HTTP vs. gRPC
Modern architectures often choose between REST and gRPC. WFB offers mirrored specifications (e.g., `JSON Aggregate` vs `gRPC Aggregate`) to provide a definitive answer on overhead and performance differences for the exact same logic.

### 3. 🛡️ Strict Validation
Speed is meaningless if the data is wrong.
Our custom load generator `wrkr` validates every single response. If a framework returns an incorrect sum in an analytics report or misses a field in a JSON object, the test fails. No caching shortcuts allowed.

### 4. 🛠️ Developer Experience
Running benchmarks shouldn't require complex ops.
WFB is a self-contained **Rust** workspace. The single CLI tool manages Docker composition, database lifecycles, and reporting.

## 🔬 Methodology & Fairness

We believe benchmarks should be transparent and reproducible.

- **Warmup Phase**: Every test includes a **30-second warmup** to allow JIT compilers (Java, C#, JS, Lua) to optimize hot paths before measurement begins.
- **Connection Stepping**: We test multiple concurrency levels (e.g., 64, 128, 256, 512 connections) to show how frameworks scale under load, rather than picking a single "magic number".
- **Realistic Client**: The `wrkr` load generator implementation uses production-grade async Rust networking (Tokio/Hyper/Reqwest) to ensure HTTP/2 and HTTP/1.1 compliance. We do not use "pipeline-optimized" synthetic clients that break spec.
- **Latency Distribution**: We capture high-resolution latency histograms (p50, p90, p99, max) to identify "hiccups" caused by GC pauses or improper async blocking.

## 🧪 Test Suite

| Test Suite | Focus | Real-World Analogy |
|------------|-------|--------------------|
| **[Plaintext](docs/specs/plaintext_spec.md)** | Baseline Throughput | Load Balancers, Gateways |
| **[JSON Analytics](docs/specs/json_aggregate_spec.md)** | CPU & Memory efficiency | Data Processing Microservices |
| **[Database Complex](docs/specs/db_complex_spec.md)** | ORM overhead, Async flows | User Dashboards, CMS |
| **[gRPC Aggregate](docs/specs/grpc_aggregate_spec.md)** | Protocol Efficiency | Inter-service Communication |
| **[Static Files](docs/specs/static_files_spec.md)** | Network I / O, Sendfile | CDNs, Asset Servers |

## 🏗 Architecture

The project is organized as a Rust workspace:

1.  **wfb-runner**: The CLI tool that orchestrates Docker containers and runs benchmarks.
2.  **wfb-server**: The API server that provides access to benchmark data.
3.  **wfb-storage**: Shared library for configuration, storage logic, and data models.
4.  **wrkr**: Custom high-performance, asynchronous load generator.

## 🏁 Quick Start

### Prerequisites

- **Rust** (2024 edition)
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

Start the API server to browse results in an interactive dashboard.

```bash
cargo run --release --bin wfb-server
# Open http://localhost:8080 in your browser
```

## 🤝 Contributing

We welcome contributions! The project is community-driven, and **anyone can add a new framework benchmark via a Pull Request**.

If you want to add your favorite framework:
1.  Read the [Adding a New Benchmark](docs/GUIDE_ADDING_BENCHMARKS.md) guide.
2.  Implement the benchmark following the specs.
3.  Submit a PR!

Whether it's adding a new framework or fixing bugs, we appreciate your help. Please check out [CONTRIBUTING.md](CONTRIBUTING.md) for general guidelines.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
