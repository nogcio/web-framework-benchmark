FROM caddy:2.10.2
COPY Caddyfile /etc/caddy/Caddyfile
COPY benchmarks_data /app/benchmarks_data

HEALTHCHECK --interval=1s --timeout=3s --retries=30 CMD wget -qO- http://localhost:8080/health || exit 1
