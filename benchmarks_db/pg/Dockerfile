FROM postgres:18

COPY init.sql /docker-entrypoint-initdb.d/

EXPOSE 5432

HEALTHCHECK --interval=5s --timeout=5s --retries=5 CMD pg_isready -U user -d hello_world || exit 1
