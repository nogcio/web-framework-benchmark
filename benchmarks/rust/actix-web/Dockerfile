FROM rust:1.92 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/wfb-rust-actix-web .
COPY benchmarks_data /app/benchmarks_data
ENV PORT=8000
CMD ["./wfb-rust-actix-web"]
