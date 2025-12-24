FROM rust:1.92 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/wfb-rust-axum-sqlx-postgres .
ENV PORT=8000
CMD ["./wfb-rust-axum-sqlx-postgres"]
