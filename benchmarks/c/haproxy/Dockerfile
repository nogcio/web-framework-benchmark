FROM haproxy:3.3.1
USER root
RUN apt-get update && apt-get install -y curl
USER haproxy
COPY haproxy.cfg /usr/local/etc/haproxy/haproxy.cfg

EXPOSE 8080

HEALTHCHECK --interval=1s --timeout=3s --retries=30 CMD curl --fail http://localhost:8080/health || exit 1
