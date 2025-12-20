FROM postgres:18

ENV POSTGRES_DB=benchmark
ENV POSTGRES_USER=benchmark
ENV POSTGRES_PASSWORD=benchmark

# Copy initialization SQL into the directory processed by the official image
COPY init.sql /docker-entrypoint-initdb.d/

EXPOSE 5432
