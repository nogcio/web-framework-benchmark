FROM nginx:1.29.4

COPY nginx.conf /etc/nginx/nginx.conf
COPY benchmarks_data /app/benchmarks_data

EXPOSE 8000

CMD ["nginx", "-g", "daemon off;"]
