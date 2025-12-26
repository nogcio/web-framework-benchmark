FROM gradle:9.2.1-jdk25 AS build
COPY --chown=gradle:gradle . /home/gradle/src
WORKDIR /home/gradle/src
RUN gradle buildFatJar --no-daemon

FROM eclipse-temurin:25.0.1_8-jdk-alpine-3.23
WORKDIR /app
COPY --from=build /home/gradle/src/build/libs/ktor-all.jar /app/ktor-all.jar
COPY benchmarks_data /app/benchmarks_data
CMD ["java", "-jar", "/app/ktor-all.jar"]
