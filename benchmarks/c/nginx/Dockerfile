FROM nginx:1.29.4

RUN apt-get update && apt-get install -y curl

COPY nginx.conf /etc/nginx/nginx.conf
COPY benchmarks_data /app/benchmarks_data

EXPOSE 8080

HEALTHCHECK --interval=1s --timeout=3s --retries=30 CMD curl --fail http://localhost:8080/health || exit 1

CMD ["nginx", "-g", "daemon off;"]
