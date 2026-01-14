FROM python:3.13.11-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends build-essential && rm -rf /var/lib/apt/lists/*

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY proto ./proto
COPY server.py ./server.py

RUN python -m grpc_tools.protoc \
    -I proto \
    --python_out=. \
    --grpc_python_out=. \
    proto/analytics.proto

COPY --from=ghcr.io/grpc-ecosystem/grpc-health-probe:v0.4.24 /ko-app/grpc-health-probe /bin/grpc_health_probe

ENV PORT=8080

EXPOSE 8080
HEALTHCHECK --interval=1s --timeout=3s --retries=30 CMD ["/bin/grpc_health_probe", "-addr=127.0.0.1:8080"]

CMD ["python", "server.py"]
