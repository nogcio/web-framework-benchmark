FROM python:3.14.2-slim

WORKDIR /app

RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY src ./src

HEALTHCHECK --interval=1s --timeout=3s --retries=30 CMD curl --fail http://localhost:8080/health || exit 1

# Use environment variable for port, default to 8080
CMD ["sh", "-c", "export WORKERS=$(nproc); export DB_POOL_SIZE=$((1000 / WORKERS)); if [ $DB_POOL_SIZE -lt 1 ]; then DB_POOL_SIZE=1; fi; uvicorn src.main:app --host 0.0.0.0 --port ${PORT:-8080} --workers $WORKERS --backlog 65535 --limit-concurrency 10000"]
