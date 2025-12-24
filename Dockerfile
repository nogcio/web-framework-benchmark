# Stage 1: Build Rust backend
FROM rust:1.92-alpine AS backend-builder
WORKDIR /app
# Install build dependencies
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static
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

# Build for release
RUN rm ./target/release/deps/wfb*
RUN cargo build --release

# Stage 2: Build Frontend
FROM node:20-alpine AS frontend-builder
WORKDIR /app

COPY web-app/package.json web-app/package-lock.json ./
RUN npm ci

COPY web-app ./
RUN npm run build

# Stage 3: Final Image
FROM alpine:3.23
WORKDIR /app

# Install necessary runtime dependencies (e.g. ca-certificates, openssl if needed)
RUN apk add --no-cache ca-certificates libssl3

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

ENTRYPOINT ["./wfb"]
