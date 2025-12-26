FROM rust:1.92 as builder
RUN apt-get update && apt-get install -y cmake clang
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/pingora_bench /usr/local/bin/pingora_bench
COPY benchmarks_data /app/benchmarks_data
CMD ["/usr/local/bin/pingora_bench"]
