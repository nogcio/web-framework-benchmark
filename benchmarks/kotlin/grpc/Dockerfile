FROM gradle:8.5.0-jdk21 AS build
COPY --chown=gradle:gradle . /home/gradle/src
WORKDIR /home/gradle/src
RUN gradle installDist --no-daemon

FROM eclipse-temurin:21-jre

# Install grpc_health_probe
ARG GRPC_HEALTH_PROBE_VERSION=v0.4.24
RUN curl -fsSL https://github.com/grpc-ecosystem/grpc-health-probe/releases/download/${GRPC_HEALTH_PROBE_VERSION}/grpc_health_probe-linux-amd64 -o /bin/grpc_health_probe && \
    chmod +x /bin/grpc_health_probe

COPY --from=build /home/gradle/src/build/install/grpc-kotlin-benchmark /app
WORKDIR /app

HEALTHCHECK --interval=5s --timeout=3s --start-period=5s --retries=3 \
    CMD /bin/grpc_health_probe -addr=:8080 || exit 1

CMD ["/app/bin/grpc-kotlin-benchmark"]
