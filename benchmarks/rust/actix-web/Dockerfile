FROM rust:1.92 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/wfb-rust-actix-web .
COPY benchmarks_data /app/benchmarks_data
ENV PORT=8080

HEALTHCHECK --interval=5s --timeout=3s --retries=3 CMD curl --fail http://localhost:8080/health || exit 1

CMD ["./wfb-rust-actix-web"]
