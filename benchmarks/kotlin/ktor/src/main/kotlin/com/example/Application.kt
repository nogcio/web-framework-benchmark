package com.example

import io.ktor.http.*
import io.ktor.serialization.kotlinx.json.*
import io.ktor.server.application.*
import io.ktor.server.engine.*
import io.ktor.server.netty.*
import io.ktor.server.plugins.contentnegotiation.*
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
    install(ContentNegotiation) {
        json(Json {
            explicitNulls = false
            ignoreUnknownKeys = true
            isLenient = true
        })
    }
    
    // X-Request-ID middleware
    intercept(ApplicationCallPipeline.Plugins) {
        val requestId = call.request.header("x-request-id")
        if (requestId != null) {
            call.response.header("x-request-id", requestId)
        }
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
        
        static("/files") {
            staticRootFolder = File(System.getenv("DATA_DIR") ?: "benchmarks_data")
            files(".")
        }

        post("/json/{from}/{to}") {
            val fromVal = call.parameters["from"]
            val toVal = call.parameters["to"]
            val root = call.receive<Root>()
            val newRoot = if (fromVal != null && toVal != null) {
                val newServlets = root.webApp.servlet.map { servlet ->
                    if (servlet.servletName == fromVal) {
                        servlet.copy(servletName = toVal)
                    } else {
                        servlet
                    }
                }
                root.copy(webApp = root.webApp.copy(servlet = newServlets))
            } else {
                root
            }
            call.respond(newRoot)
        }
    }
}

@Serializable
data class Root(
    @SerialName("web-app") val webApp: WebApp
)

@Serializable
data class WebApp(
    val servlet: List<Servlet>,
    @SerialName("servlet-mapping") val servletMapping: Map<String, String>,
    val taglib: Taglib
)

@Serializable
data class Servlet(
    @SerialName("servlet-name") val servletName: String,
    @SerialName("servlet-class") val servletClass: String,
    @SerialName("init-param") val initParam: JsonObject? = null
)

@Serializable
data class Taglib(
    @SerialName("taglib-uri") val taglibUri: String,
    @SerialName("taglib-location") val taglibLocation: String
)
