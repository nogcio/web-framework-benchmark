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
CMD ["./main"]
