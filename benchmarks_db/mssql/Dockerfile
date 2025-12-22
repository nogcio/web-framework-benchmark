FROM mcr.microsoft.com/mssql/server:2025-latest

COPY init.sql /docker-entrypoint-initdb.d/
COPY --chmod=755 entrypoint.sh /usr/config/entrypoint.sh
COPY --chmod=755 check_ready.sh /usr/config/check_ready.sh

EXPOSE 1433

ENTRYPOINT ["/usr/config/entrypoint.sh"]
CMD ["/opt/mssql/bin/sqlservr"]
