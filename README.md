# Web Framework Benchmark (WFB)

A comprehensive benchmarking tool for comparing the performance of web frameworks and HTTP services across different programming languages. This project provides automated benchmarking infrastructure, result visualization, and extensible framework support.

## Features

- **Multi-language Support**: Benchmark frameworks written in different languages (currently Go, extensible to others)
- **Comprehensive Test Suite**: Includes tests for:
  - Hello World responses
  - JSON serialization/deserialization
  - Database read operations (single and paginated)
  - Database write operations
  - Static file serving
- **Automated Benchmarking**: Uses `wrk` for high-performance HTTP load testing
- **Result Visualization**: Web dashboard built with React/TypeScript for viewing and comparing results
- **Result Storage**: Local filesystem storage of benchmark results in YAML format
- **Database Integration**: PostgreSQL test database for benchmark workloads
- **Docker Support**: Containerized environments for consistent benchmarking
- **Local and Remote Environments**: Support for both local development and remote deployment

## Architecture

The project consists of several components:

- **Rust CLI (`src/`)**: Core benchmarking engine and command-line interface
- **Web Dashboard (`web-app/`)**: React application for result visualization
- **Framework Implementations (`benchmarks/`)**: Example web services in different languages/frameworks
- **Database (`benchmarks_db/`)**: PostgreSQL setup with initialization scripts for test data
- **Configuration (`config/`)**: Language and environment configurations
- **Scripts (`scripts/`)**: Lua scripts for `wrk` load testing
- **Test Data (`benchmarks_data/`)**: Static files for benchmarking

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

### 2. View CLI Help

```bash
cargo run --release -- --help
```

### 3. Run a Benchmark

```bash
# Benchmark a specific framework
cargo run --release -- benchmark benchmarks/go/std --environment local

# Run all benchmarks and save to database
cargo run --release -- run 1 --environment local
```

### 4. Start the Web Dashboard

```bash
cd web-app
npm install
npm run dev
```

Open http://localhost:5173 to view the dashboard.

### 5. Database Setup (Optional - for database benchmarks)

```bash
cd benchmarks_db
docker build -t wfb-db .
docker run -d -p 5432:5432 --name wfb-db wfb-db
```

## Usage

### CLI Commands

#### Benchmark Command
Run benchmarks for a specific framework implementation:

```bash
cargo run --release -- benchmark <path> [--environment <type>]
```

- `path`: Path to the framework implementation directory
- `environment`: `local` (default) or `remote`

#### Run Command
Execute all configured benchmarks and store results:

```bash
cargo run --release -- run <id> [--environment <type>]
```

- `id`: Unique run identifier
- `environment`: `local` (default) or `remote`

### Adding New Frameworks

1. Create a new directory under `benchmarks/<language>/<framework>/`
2. Implement the required endpoints (see existing implementations for reference)
3. Add configuration to `config/languages.yaml`
4. Ensure Docker support if needed

### Required Endpoints

Framework implementations must provide these endpoints:

- `GET /` - Hello World response
- `GET /info` - Server version and supported tests information (returns plain text: `version,test1,test2,...` where version is a string and tests are from: hello_world, json, db_read_one, db_read_paging, db_write, static_files)
- `GET /json` - JSON serialization
- `GET /db` - Single database read
- `GET /db/paging?page=1&limit=10` - Paginated database read
- `POST /db` - Database write
- `GET /static/*` - Static file serving

## Configuration

### Languages Configuration (`config/languages.yaml`)

Define supported languages and frameworks:

```yaml
- name: Go
  url: https://golang.org
  frameworks:
    - name: stdlib
      path: benchmarks/go/std
      url: https://golang.org/pkg/net/http/
      tags:
        go: "1.21"
        platform: go
```

### Environment Configuration (`config/environment.local.yaml`)

Configure local benchmarking parameters:

```yaml
# Local environment settings
docker_network: wfb-network
database:
  host: db
  port: 5432
  user: benchmark
  password: benchmark
  name: benchmark
wrk:
  duration: 10s
  threads: 4
  connections: 100
```

## Development

### Code Quality

```bash
# Rust formatting and linting
cargo fmt --all
cargo clippy --all-targets -- -D warnings

# Frontend linting
cd web-app
npm run lint
```

### Testing

```bash
# Run Rust tests
cargo test

# Run frontend tests (if configured)
cd web-app
npm test
```

### Building for Production

```bash
# Build Rust CLI
cargo build --release

# Build web app
cd web-app
npm run build
```

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

### Adding Support for New Languages/Frameworks

1. Fork the repository
2. Create a new framework implementation in `benchmarks/<language>/<framework>/`
3. Update `config/languages.yaml`
4. Add Docker configuration if needed
5. Test locally
6. Submit a pull request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Authors

- **Andrew Sumskoy** - *Initial work* - [getansum@nogc.io](mailto:getansum@nogc.io)

## Acknowledgments

- Built with [Rust](https://www.rust-lang.org/), [React](https://reactjs.org/), and [Vite](https://vitejs.dev/)
- Load testing powered by [wrk](https://github.com/wg/wrk)
- UI components from [Radix UI](https://www.radix-ui.com/) and [Tailwind CSS](https://tailwindcss.com/)
