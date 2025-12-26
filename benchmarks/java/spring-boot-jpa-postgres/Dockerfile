FROM maven:3.9.9-eclipse-temurin-23 AS build
WORKDIR /app

COPY pom.xml .
RUN mvn -q -DskipTests dependency:go-offline

COPY src ./src
RUN mvn -q -DskipTests package

FROM eclipse-temurin:23-jre
WORKDIR /app

COPY --from=build /app/target/app.jar /app/app.jar
COPY benchmarks_data /app/benchmarks_data

EXPOSE 8000
ENV PORT=8000

CMD ["java", "-XX:+AlwaysPreTouch", "-Djava.security.egd=file:/dev/./urandom", "-jar", "/app/app.jar"]
