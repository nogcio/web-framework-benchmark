# Build Stage
FROM rust:1.92 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Runtime Stage
FROM debian:bookworm-slim
# Install curl for healthcheck and libpq for diesel
RUN apt-get update && apt-get install -y curl libpq5 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/wfb-rust-axum-diesel-postgres .

ENV PORT=8080

# MANDATORY: Healthcheck
HEALTHCHECK --interval=5s --timeout=3s --retries=3 \
  CMD curl --fail http://localhost:8080/health || exit 1

CMD ["./wfb-rust-axum-diesel-postgres"]
