package org.acme.grpc

import io.grpc.ServerBuilder
import io.grpc.protobuf.services.HealthStatusManager

fun main() {
    val port = System.getenv("PORT")?.toIntOrNull() ?: 8080
    val healthStatusManager = HealthStatusManager()
    val server = ServerBuilder.forPort(port)
        .intercept(ClientIdInterceptor())
        .addService(AnalyticsService())
        .addService(healthStatusManager.healthService)
        .build()

    println("Starting gRPC server on port $port")
    server.start()
    Runtime.getRuntime().addShutdownHook(Thread {
        println("Shutting down gRPC server")
        server.shutdown()
    })
    server.awaitTermination()
}
