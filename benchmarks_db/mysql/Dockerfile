FROM mysql:9.5.0

COPY init.sql /docker-entrypoint-initdb.d/
COPY max-connections.cnf /etc/mysql/conf.d/

HEALTHCHECK --interval=5s --timeout=5s --retries=5 CMD mysqladmin ping -h localhost -u ${MYSQL_USER} -p${MYSQL_PASSWORD} || exit 1

EXPOSE 3306
