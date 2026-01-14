FROM golang:1.25.5-alpine AS builder

WORKDIR /app
COPY . .
RUN go build -ldflags="-s -w" -o server main.go

FROM alpine:latest
WORKDIR /app
COPY --from=builder /app/server .
ENV PORT=8080
HEALTHCHECK --interval=1s --timeout=3s --retries=30 CMD wget -qO- http://localhost:8080/health || exit 1
CMD ["./server"]
