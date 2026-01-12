#!/bin/bash
set -euo pipefail

SQLCMD=/opt/mssql-tools18/bin/sqlcmd
if [ ! -x "$SQLCMD" ]; then
  SQLCMD=/opt/mssql-tools/bin/sqlcmd
fi

MSSQL_SA_PASSWORD=${MSSQL_SA_PASSWORD:-Benchmark!12345}

exec "$SQLCMD" -C -S 127.0.0.1,1433 -U sa -P "$MSSQL_SA_PASSWORD" -Q "IF (SELECT COUNT(*) FROM hello_world.dbo.users) >= 10000 SELECT 1 ELSE THROW 51000, 'Not ready', 1;" >/dev/null
