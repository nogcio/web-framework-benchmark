FROM python:3.14.2-slim

WORKDIR /app

# aiohttp may compile native extensions when wheels are unavailable (e.g. new Python/aarch64).
RUN apt-get update && apt-get install -y curl gcc && rm -rf /var/lib/apt/lists/*

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY src ./src
COPY benchmarks_data /app/benchmarks_data

HEALTHCHECK --interval=1s --timeout=3s --retries=30 CMD curl --fail http://localhost:8080/health || exit 1

CMD ["sh", "-c", "python -u src/main.py"]
