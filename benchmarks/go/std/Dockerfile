# syntax=docker/dockerfile:1

FROM golang:1.21 AS builder
WORKDIR /app
COPY go.mod ./
RUN go mod download
COPY src/ ./src/
RUN go mod tidy
RUN CGO_ENABLED=0 go build -ldflags="-s -w" -o wfb_go_std ./src

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/wfb_go_std /app/wfb_go_std
EXPOSE 8000
CMD ["/app/wfb_go_std"]