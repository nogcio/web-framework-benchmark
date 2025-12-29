FROM mysql:9.5.0

ENV MYSQL_DATABASE=benchmark
ENV MYSQL_USER=benchmark
ENV MYSQL_PASSWORD=benchmark
ENV MYSQL_ROOT_PASSWORD=benchmark

COPY init.sql /docker-entrypoint-initdb.d/
COPY max-connections.cnf /etc/mysql/conf.d/

HEALTHCHECK --interval=5s --timeout=5s --retries=5 CMD mysqladmin ping -h localhost -u ${MYSQL_USER} -p${MYSQL_PASSWORD} || exit 1

EXPOSE 3306
