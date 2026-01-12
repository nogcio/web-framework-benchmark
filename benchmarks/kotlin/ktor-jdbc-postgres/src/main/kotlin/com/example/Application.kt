package com.example

import com.zaxxer.hikari.HikariConfig
import com.zaxxer.hikari.HikariDataSource
import io.ktor.http.*
import io.ktor.serialization.kotlinx.json.*
import io.ktor.server.application.*
import io.ktor.server.auth.*
import io.ktor.server.engine.*
import io.ktor.server.netty.*
import io.ktor.server.plugins.contentnegotiation.*
import io.ktor.server.request.*
import io.ktor.server.response.*
import io.ktor.server.routing.*
import kotlinx.serialization.Serializable
import java.sql.ResultSet
import java.time.LocalDateTime

fun main() {
    val port = System.getenv("PORT")?.toIntOrNull() ?: 8080
    embeddedServer(Netty, port = port, host = "0.0.0.0", module = Application::module)
        .start(wait = true)
}

fun Application.module() {
    val dbHost = System.getenv("DB_HOST") ?: "localhost"
    val dbPort = System.getenv("DB_PORT") ?: "5432"
    val dbName = System.getenv("DB_NAME") ?: "benchmark"
    val dbUser = System.getenv("DB_USER") ?: "benchmark"
    val dbPass = System.getenv("DB_PASSWORD") ?: "benchmark"
    val dbPoolSize = System.getenv("DB_POOL_SIZE")?.toIntOrNull() ?: 256

    val config = HikariConfig().apply {
        jdbcUrl = "jdbc:postgresql://$dbHost:$dbPort/$dbName"
        username = dbUser
        password = dbPass
        maximumPoolSize = dbPoolSize
        minimumIdle = dbPoolSize
        driverClassName = "org.postgresql.Driver"
        addDataSourceProperty("cachePrepStmts", "true")
        addDataSourceProperty("prepStmtCacheSize", "250")
        addDataSourceProperty("prepStmtCacheSqlLimit", "2048")
    }
    val dataSource = HikariDataSource(config)

    install(ContentNegotiation) {
        json()
    }

    routing {
        get("/health") {
            try {
                dataSource.connection.use { conn ->
                    conn.createStatement().use { stmt ->
                        stmt.execute("SELECT 1")
                    }
                }
                call.respondText("OK")
            } catch (e: Exception) {
                call.respond(HttpStatusCode.InternalServerError, "Database unavailable")
            }
        }

        get("/db/read/one") {
            val id = call.request.queryParameters["id"]?.toIntOrNull() ?: 1
            dataSource.connection.use { conn ->
                conn.prepareStatement("SELECT id, name, created_at, updated_at FROM hello_world WHERE id = ?").use { stmt ->
                    stmt.setInt(1, id)
                    stmt.executeQuery().use { rs ->
                        if (rs.next()) {
                            call.respond(rs.toHelloWorld())
                        } else {
                            call.respond(HttpStatusCode.NotFound)
                        }
                    }
                }
            }
        }

        get("/db/read/many") {
            val offset = call.request.queryParameters["offset"]?.toIntOrNull() ?: 0
            val limit = call.request.queryParameters["limit"]?.toIntOrNull() ?: 50
            val list = ArrayList<HelloWorld>(limit)
            dataSource.connection.use { conn ->
                conn.prepareStatement("SELECT id, name, created_at, updated_at FROM hello_world ORDER BY id LIMIT ? OFFSET ?").use { stmt ->
                    stmt.setInt(1, limit)
                    stmt.setInt(2, offset)
                    stmt.executeQuery().use { rs ->
                        while (rs.next()) {
                            list.add(rs.toHelloWorld())
                        }
                    }
                }
            }
            call.respond(list)
        }

        post("/db/write/insert") {
            val payload = call.receive<WritePayload>()

            val now = java.time.LocalDateTime.now()
            
            dataSource.connection.use { conn ->
                conn.prepareStatement("INSERT INTO hello_world (name, created_at, updated_at) VALUES (?, ?, ?) RETURNING id, name, created_at, updated_at").use { stmt ->
                    stmt.setString(1, payload.name)
                    stmt.setObject(2, now)
                    stmt.setObject(3, now)
                    stmt.executeQuery().use { rs ->
                        if (rs.next()) {
                            call.respond(HelloWorld(
                                id = rs.getInt("id"),
                                name = rs.getString("name"),
                                createdAt = rs.getTimestamp("created_at").toInstant().toString(),
                                updatedAt = rs.getTimestamp("updated_at").toInstant().toString()
                            ))
                        }
                    }
                }
            }
        }
    }
}

fun ResultSet.toHelloWorld() = HelloWorld(
    id = getInt("id"),
    name = getString("name"),
    createdAt = getTimestamp("created_at").toInstant().toString(),
    updatedAt = getTimestamp("updated_at").toInstant().toString()
)

@Serializable
data class HelloWorld(val id: Int, val name: String, val createdAt: String, val updatedAt: String)

@Serializable
data class WritePayload(val name: String)

@Serializable
data class AuthRequest(val username: String, val password: String)

@Serializable
data class Tweet(val id: Int, val username: String, val content: String, val createdAt: String, val likes: Int)

@Serializable
data class CreateTweetRequest(val content: String)
