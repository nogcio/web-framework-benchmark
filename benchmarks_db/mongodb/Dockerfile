FROM mongo:8.2.3

COPY init.js /docker-entrypoint-initdb.d/

EXPOSE 27017

HEALTHCHECK --interval=5s --timeout=5s --retries=5 CMD mongosh --eval "db.adminCommand('ping')" || exit 1
