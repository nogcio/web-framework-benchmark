# Stage 1: Build Rust backend
FROM rust:1.92-slim-bookworm as backend-builder
WORKDIR /app
# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
# Create a new empty shell project
RUN cargo new --bin wfb
WORKDIR /app/wfb

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Build only the dependencies to cache them
RUN cargo build --release
RUN rm src/*.rs

# Copy the source code
COPY src ./src
# Copy other necessary files for compilation if any
# (None identified as strictly necessary for compilation, but config might be needed at runtime)

# Build for release
RUN rm ./target/release/deps/wfb*
RUN cargo build --release

# Stage 2: Build Frontend
FROM node:20-slim as frontend-builder
WORKDIR /app

COPY web-app/package.json web-app/package-lock.json ./
RUN npm ci

COPY web-app ./
RUN npm run build

# Stage 3: Final Image
FROM debian:bookworm-slim
WORKDIR /app

# Install necessary runtime dependencies (e.g. ca-certificates, openssl if needed)
# Also install docker CLI so the app can spawn sibling containers
RUN apt-get update && apt-get install -y ca-certificates docker.io && rm -rf /var/lib/apt/lists/*

# Copy the binary
COPY --from=backend-builder /app/wfb/target/release/wfb .

# Copy the frontend build to static
COPY --from=frontend-builder /app/dist ./static

# Copy configuration and other assets if needed
COPY config ./config
COPY benchmarks ./benchmarks
COPY benchmarks_db ./benchmarks_db
COPY scripts ./scripts

# Expose port
EXPOSE 8080

CMD ["./wfb", "serve", "--host", "0.0.0.0", "--port", "8080"]
