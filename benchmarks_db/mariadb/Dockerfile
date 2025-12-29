FROM mariadb:12.1.2

ENV MARIADB_DATABASE=benchmark
ENV MARIADB_USER=benchmark
ENV MARIADB_PASSWORD=benchmark
ENV MARIADB_ROOT_PASSWORD=benchmark

COPY init.sql /docker-entrypoint-initdb.d/
COPY max-connections.cnf /etc/mysql/conf.d/

HEALTHCHECK --interval=5s --timeout=5s --retries=5 CMD mysqladmin ping -h localhost -u ${MARIADB_USER} -p${MARIADB_PASSWORD} || exit 1

EXPOSE 3306
