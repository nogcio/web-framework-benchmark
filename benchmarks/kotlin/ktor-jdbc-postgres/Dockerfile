FROM gradle:9.2.1-jdk25 AS build
COPY --chown=gradle:gradle . /home/gradle/src
WORKDIR /home/gradle/src
RUN gradle buildFatJar --no-daemon

FROM eclipse-temurin:25.0.1_8-jdk-alpine-3.23

# Install curl for healthcheck
RUN apk add --no-cache curl

WORKDIR /app
COPY --from=build /home/gradle/src/build/libs/ktor-all.jar /app/ktor-all.jar

ENV PORT=8080

HEALTHCHECK --interval=1s --timeout=3s --retries=30 \
  CMD curl --fail http://localhost:${PORT}/health || exit 1

CMD ["java", "-jar", "/app/ktor-all.jar"]
