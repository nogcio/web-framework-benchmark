FROM debian:12-slim AS build

RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    pkg-config \
    protobuf-compiler \
    protobuf-compiler-grpc \
    libprotobuf-dev \
    libgrpc++-dev \
    libjemalloc-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cmake -S . -B build -DCMAKE_BUILD_TYPE=Release && \
    cmake --build build -j

FROM debian:12-slim
RUN apt-get update && apt-get install -y \
    libgrpc++-dev \
    libprotobuf-dev \
    ca-certificates \
    libjemalloc2 \
    && rm -rf /var/lib/apt/lists/*

ENV LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libjemalloc.so.2

WORKDIR /app
COPY --from=build /app/build/server ./server
COPY --from=ghcr.io/grpc-ecosystem/grpc-health-probe:v0.4.24 /ko-app/grpc-health-probe /bin/grpc_health_probe

ENV PORT=8080

EXPOSE 8080
HEALTHCHECK --interval=5s --timeout=3s --retries=3 CMD ["/bin/grpc_health_probe", "-addr=127.0.0.1:8080"]

CMD ["./server"]
