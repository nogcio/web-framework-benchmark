FROM golang:1.25.5-alpine AS builder
WORKDIR /app
COPY . .
RUN go mod tidy
RUN go build -o main .

FROM alpine:latest
WORKDIR /app
COPY --from=builder /app/main .
COPY benchmarks_data /app/benchmarks_data
CMD ["./main"]
