FROM golang:1.25-alpine AS builder

WORKDIR /app
COPY . .
RUN go build -ldflags="-s -w" -o server main.go

FROM alpine:3.21
WORKDIR /app
COPY --from=builder /app/server .
ENV PORT=8080
HEALTHCHECK --interval=5s --timeout=3s --retries=3 CMD wget -qO- http://localhost:8080/health || exit 1
CMD ["./server"]
