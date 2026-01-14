FROM python:3.14.2-slim

WORKDIR /app

# Install system dependencies (curl for healthcheck)
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY . .
COPY benchmarks_data /app/benchmarks_data

ENV PORT=8080
ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1
ENV DJANGO_SETTINGS_MODULE=wfb.settings

# Run gunicorn
# Use nproc to determine workers count, similar to other Python frameworks
CMD ["sh", "-c", "export WORKERS=$(nproc); gunicorn wfb.wsgi:application --bind 0.0.0.0:8080 --workers $WORKERS --threads 4"]

HEALTHCHECK --interval=1s --timeout=3s --retries=30 \
  CMD curl --fail http://localhost:8080/health || exit 1
