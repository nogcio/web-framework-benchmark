FROM python:3.14.2-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY src ./src

# Use environment variable for port, default to 8000
CMD ["sh", "-c", "export WORKERS=$(nproc); export DB_POOL_SIZE=$((256 / WORKERS)); if [ $DB_POOL_SIZE -lt 1 ]; then DB_POOL_SIZE=1; fi; uvicorn src.main:app --host 0.0.0.0 --port ${PORT:-8000} --workers $WORKERS"]
