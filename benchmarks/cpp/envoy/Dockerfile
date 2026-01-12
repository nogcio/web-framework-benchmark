FROM envoyproxy/envoy:v1.36.4

RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*

COPY envoy.yaml /etc/envoy/envoy.yaml

HEALTHCHECK --interval=5s --timeout=3s --retries=3 CMD curl --fail http://localhost:8080/health || exit 1

CMD ["/usr/local/bin/envoy", "-c", "/etc/envoy/envoy.yaml", "--service-cluster", "front-proxy"]
