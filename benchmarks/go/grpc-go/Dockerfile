FROM golang:1.25-bookworm AS builder

# Install protoc
RUN apt-get update && apt-get install -y protobuf-compiler

WORKDIR /app

# Install protoc-gen-go and protoc-gen-go-grpc
RUN go install google.golang.org/protobuf/cmd/protoc-gen-go@v1.36.11
RUN go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@v1.6.0

COPY go.mod ./
RUN go mod download

COPY . .

# Generate proto code
RUN protoc --go_out=. --go_opt=paths=source_relative \
    --go-grpc_out=. --go-grpc_opt=paths=source_relative \
    proto/analytics.proto

# Tidy deps after code generation (to pick up imports from generated code)
RUN go mod tidy

# Build
RUN CGO_ENABLED=0 GOOS=linux go build -o wfb-go-grpc main.go

# Install grpc-health-probe
RUN CGO_ENABLED=0 go install github.com/grpc-ecosystem/grpc-health-probe@v0.4.24 && \
    cp /go/bin/grpc-health-probe /bin/grpc_health_probe

FROM scratch
WORKDIR /app
COPY --from=builder /app/wfb-go-grpc .

# Add grpc-health-probe
COPY --from=builder /bin/grpc_health_probe /bin/grpc_health_probe

ENV PORT=8080
ENV GOGC=1000

EXPOSE 8080

HEALTHCHECK --interval=5s --timeout=3s --retries=3 CMD ["/bin/grpc_health_probe", "-addr=127.0.0.1:8080"]

CMD ["./wfb-go-grpc"]
