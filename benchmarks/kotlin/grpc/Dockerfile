FROM gradle:9.2.1-jdk25 AS build
COPY --chown=gradle:gradle . /home/gradle/src
WORKDIR /home/gradle/src
RUN gradle installDist --no-daemon

FROM eclipse-temurin:25-jre

# Install grpc_health_probe
COPY --from=ghcr.io/grpc-ecosystem/grpc-health-probe:v0.4.24 /ko-app/grpc-health-probe /bin/grpc_health_probe

COPY --from=build /home/gradle/src/build/install/grpc-kotlin-benchmark /app
WORKDIR /app

HEALTHCHECK --interval=1s --timeout=3s --retries=30 \
    CMD /bin/grpc_health_probe -addr=:8080 || exit 1

CMD ["/app/bin/grpc-kotlin-benchmark"]
