FROM python:3.14-slim

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
# 4 workers, 4 threads is a reasonable default for containerized python apps
CMD ["gunicorn", "wfb.wsgi:application", "--bind", "0.0.0.0:8080", "--workers", "4", "--threads", "4"]

HEALTHCHECK --interval=5s --timeout=3s --retries=3 \
  CMD curl --fail http://localhost:8080/health || exit 1
