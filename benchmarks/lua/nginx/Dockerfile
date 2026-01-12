FROM openresty/openresty:1.27.1.2-0-alpine

RUN apk add --no-cache curl

COPY nginx.conf /usr/local/openresty/nginx/conf/nginx.conf

EXPOSE 8080

HEALTHCHECK --interval=5s --timeout=3s --retries=3 CMD curl --fail http://localhost:8080/health || exit 1
