package com.example

import io.ktor.http.*
import io.ktor.serialization.kotlinx.json.*
import io.ktor.server.application.*
import io.ktor.server.engine.*
import io.ktor.server.netty.*
import io.ktor.server.plugins.contentnegotiation.*
import io.ktor.server.plugins.autohead.*
import io.ktor.server.plugins.partialcontent.*
import io.ktor.server.request.*
import io.ktor.server.response.*
import io.ktor.server.routing.*
import io.ktor.server.http.content.*
import kotlinx.serialization.*
import kotlinx.serialization.json.*
import java.io.File

fun main() {
    val port = System.getenv("PORT")?.toIntOrNull() ?: 8080
    embeddedServer(Netty, port = port, host = "0.0.0.0", module = Application::module)
        .start(wait = true)
}

fun Application.module() {
    install(AutoHeadResponse)
    install(PartialContent)
    install(ContentNegotiation) {
        json(Json {
            explicitNulls = false
            ignoreUnknownKeys = true
            isLenient = true
        })
    }

    routing {
        get("/health") {
            call.respondText("OK")
        }

        get("/") {
            call.respondText("Hello, World!")
        }

        get("/plaintext") {
            call.respondText("Hello, World!")
        }
        
        staticFiles("/files", File(System.getenv("DATA_DIR") ?: "benchmarks_data")) {
            contentType { file ->
                if (file.extension == "bin") {
                    ContentType.Application.OctetStream
                } else {
                    null
                }
            }
        }

        post("/json/aggregate") {
            val orders = call.receive<List<Order>>()
            var processedOrders = 0
            val results = mutableMapOf<String, Long>()
            val categoryStats = mutableMapOf<String, Int>()

            for (order in orders) {
                if (order.status == "completed") {
                    processedOrders++
                    results[order.country] = (results[order.country] ?: 0L) + order.amount
                    
                    order.items?.forEach { item ->
                        categoryStats[item.category] = (categoryStats[item.category] ?: 0) + item.quantity
                    }
                }
            }

            call.respond(AggregateResponse(processedOrders, results, categoryStats))
        }
    }
}

@Serializable
data class Order(
    val status: String,
    val amount: Long,
    val country: String,
    val items: List<OrderItem>? = null
)

@Serializable
data class OrderItem(
    val quantity: Int,
    val category: String
)

@Serializable
data class AggregateResponse(
    @SerialName("processedOrders") val processedOrders: Int,
    val results: Map<String, Long>,
    @SerialName("categoryStats") val categoryStats: Map<String, Int>
)