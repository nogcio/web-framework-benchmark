FROM golang:1.25.5-alpine AS builder
WORKDIR /app
COPY . .
RUN go mod tidy
RUN go build -tags=go_json -o main .

FROM alpine:latest
WORKDIR /app
COPY --from=builder /app/main .
COPY benchmarks_data /app/benchmarks_data
ENV GIN_MODE=release

HEALTHCHECK --interval=1s --timeout=3s --retries=30 CMD wget -qO- http://localhost:8080/health || exit 1

CMD ["./main"]
