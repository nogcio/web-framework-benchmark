# syntax=docker/dockerfile:1

FROM golang:1.21 AS builder
WORKDIR /app
COPY go.mod ./
RUN go mod download
COPY src/ ./src/
RUN go mod tidy
RUN go build -o wfb_go_gin ./src

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/wfb_go_gin /app/wfb_go_gin
EXPOSE 8000
CMD ["/app/wfb_go_gin"]