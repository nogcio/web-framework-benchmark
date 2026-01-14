FROM maven:3.9.12-eclipse-temurin-25 AS builder
WORKDIR /app
COPY pom.xml .
RUN mvn -B dependency:resolve dependency:resolve-plugins
COPY src ./src
RUN mvn -B package -DskipTests -Dquarkus.package.jar.type=uber-jar

FROM eclipse-temurin:25-jre-alpine
WORKDIR /app
COPY --from=builder /app/target/*-runner.jar app.jar

COPY --from=ghcr.io/grpc-ecosystem/grpc-health-probe:v0.4.24 /ko-app/grpc-health-probe /bin/grpc_health_probe

ENV QUARKUS_GRPC_SERVER_PORT=8080
ENV QUARKUS_HTTP_PORT=8081
ENV QUARKUS_GRPC_SERVER_HOST=0.0.0.0

HEALTHCHECK --interval=1s --timeout=3s --retries=30 CMD ["/bin/grpc_health_probe", "-addr=127.0.0.1:8080"]

CMD ["java", "-XX:MaxRAMPercentage=75.0", "-jar", "app.jar"]
