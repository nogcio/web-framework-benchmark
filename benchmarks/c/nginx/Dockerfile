FROM openresty/openresty:1.27.1.2-0-alpine

RUN apk add --no-cache curl

COPY nginx.conf /usr/local/openresty/nginx/conf/nginx.conf
COPY benchmarks_data /app/benchmarks_data

EXPOSE 8080

HEALTHCHECK --interval=1s --timeout=3s --retries=30 CMD curl --fail http://localhost:8080/health || exit 1

CMD ["/usr/local/openresty/bin/openresty", "-g", "daemon off;"]
