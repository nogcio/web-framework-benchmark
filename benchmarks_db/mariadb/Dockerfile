FROM mariadb:12.1.2

ENV MARIADB_DATABASE=benchmark
ENV MARIADB_USER=benchmark
ENV MARIADB_PASSWORD=benchmark
ENV MARIADB_ROOT_PASSWORD=benchmark

COPY init.sql /docker-entrypoint-initdb.d/
COPY max-connections.cnf /etc/mysql/conf.d/

EXPOSE 3306
