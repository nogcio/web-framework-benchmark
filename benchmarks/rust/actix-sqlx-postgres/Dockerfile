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
COPY --from=builder /app/target/release/wfb-rust-actix-sqlx-postgres .

ENV PORT=8080

# MANDATORY: Healthcheck
HEALTHCHECK --interval=1s --timeout=3s --retries=30 \
  CMD curl --fail http://localhost:8080/health || exit 1

CMD ["./wfb-rust-actix-sqlx-postgres"]
