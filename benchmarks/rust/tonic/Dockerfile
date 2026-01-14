FROM rust:1.92 AS builder
WORKDIR /app
COPY . .
RUN apt-get update && apt-get install -y protobuf-compiler cmake
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/wfb-rust-tonic .
ENV PORT=8080

RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/* \
    && curl -L https://github.com/grpc-ecosystem/grpc-health-probe/releases/download/v0.4.19/grpc_health_probe-linux-amd64 -o /bin/grpc_health_probe \
    && chmod +x /bin/grpc_health_probe

HEALTHCHECK --interval=1s --timeout=3s --retries=30 CMD /bin/grpc_health_probe -addr=localhost:8080 || exit 1

CMD ["./wfb-rust-tonic"]
