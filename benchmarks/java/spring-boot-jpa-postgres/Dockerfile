FROM maven:3.9.9-eclipse-temurin-23 AS build
WORKDIR /app

COPY pom.xml .
RUN mvn -q -DskipTests dependency:go-offline

COPY src ./src
RUN mvn -q -DskipTests package

FROM eclipse-temurin:23-jre
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
WORKDIR /app

COPY --from=build /app/target/app.jar /app/app.jar
COPY benchmarks_data /app/benchmarks_data

EXPOSE 8080
ENV PORT=8080

HEALTHCHECK --interval=5s --timeout=3s --retries=3 CMD curl --fail http://localhost:8080/health || exit 1

CMD ["java", "-XX:+AlwaysPreTouch", "-Djava.security.egd=file:/dev/./urandom", "-jar", "/app/app.jar"]
