#!/bin/bash
set -euo pipefail

/opt/mssql/bin/sqlservr &
MSSQL_PID=${MSSQL_PID:-Developer}
MSSQL_SA_PASSWORD=${MSSQL_SA_PASSWORD:-Benchmark!12345}

# sqlcmd path changed in newer images
SQLCMD=/opt/mssql-tools18/bin/sqlcmd
if [ ! -x "$SQLCMD" ]; then
  SQLCMD=/opt/mssql-tools/bin/sqlcmd
fi
SQLCMD_OPTS=(-C -S localhost -U sa -P "$MSSQL_SA_PASSWORD")

# Wait for SQL Server to accept connections
for i in {1..90}; do
  if "$SQLCMD" "${SQLCMD_OPTS[@]}" -Q "SELECT 1" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

# Run initialization script if present
if [ -f /docker-entrypoint-initdb.d/init.sql ]; then
  "$SQLCMD" "${SQLCMD_OPTS[@]}" -i /docker-entrypoint-initdb.d/init.sql
fi

wait
