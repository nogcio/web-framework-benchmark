FROM rust:1.92 as builder
RUN apt-get update && apt-get install -y cmake clang
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/pingora_bench /usr/local/bin/pingora_bench
COPY benchmarks_data /app/benchmarks_data

HEALTHCHECK --interval=5s --timeout=3s --retries=3 CMD curl --fail http://localhost:8080/health || exit 1

CMD ["/usr/local/bin/pingora_bench"]
