FROM mariadb:12.1.2

COPY init.sql /docker-entrypoint-initdb.d/
COPY max-connections.cnf /etc/mysql/conf.d/

HEALTHCHECK --interval=5s --timeout=5s --retries=5 CMD mariadb-admin ping -h localhost -u ${MARIADB_USER} -p${MARIADB_PASSWORD} || exit 1

EXPOSE 3306
