# Stage 1: Build Rust backend
FROM rust:1.92-alpine AS backend-builder
WORKDIR /app
# Install build dependencies
RUN apk add --no-cache musl-dev pkgconfig openssl-dev libssh2-dev luajit-dev openssl-libs-static zlib-static libssh2-static build-base nodejs npm

# Copy workspace configuration
COPY Cargo.toml Cargo.lock ./

# Copy crate directories
COPY wfb-runner ./wfb-runner
COPY wfb-server ./wfb-server
COPY wfb-storage ./wfb-storage
COPY wrkr ./wrkr
COPY wrkr-api ./wrkr-api
COPY wrkr-core ./wrkr-core

# Build wfb targets
RUN RUSTFLAGS="-C link-arg=-lgcc" cargo build --release -p wfb-runner -p wfb-server

# Stage 2: Final Image
FROM alpine:3.23
WORKDIR /app

# Install necessary runtime dependencies
RUN apk add --no-cache ca-certificates libgcc libssh2 openssl luajit

# Copy the binaries to /usr/local/bin so they are in PATH
COPY --from=backend-builder /app/target/release/wfb-runner /usr/local/bin/wfb-runner
COPY --from=backend-builder /app/target/release/wfb-server /usr/local/bin/wfb-server

# Copy configuration and other assets
COPY config ./config
COPY benchmarks ./benchmarks
COPY benchmarks_db ./benchmarks_db
COPY scripts ./scripts

# Copy built web assets from wfb-server build
COPY --from=backend-builder /app/wfb-server/assets/dist ./assets

# Default to running the server
CMD ["wfb-server"]
