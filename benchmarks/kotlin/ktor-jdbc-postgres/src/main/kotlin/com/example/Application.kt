package com.example

import com.zaxxer.hikari.HikariConfig
import com.zaxxer.hikari.HikariDataSource
import io.ktor.http.*
import io.ktor.serialization.kotlinx.json.*
import io.ktor.server.application.*
import io.ktor.server.engine.*
import io.ktor.server.netty.*
import io.ktor.server.plugins.contentnegotiation.*
import io.ktor.server.response.*
import io.ktor.server.routing.*
import kotlinx.coroutines.*
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import java.sql.ResultSet

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
        json(Json { 
            ignoreUnknownKeys = true 
            encodeDefaults = true
        })
    }

    routing {
        get("/health") {
            try {
                withContext(Dispatchers.IO) {
                    dataSource.connection.use { conn ->
                        conn.createStatement().use { stmt ->
                            stmt.execute("SELECT 1")
                        }
                    }
                }
                call.respondText("OK")
            } catch (e: Exception) {
                call.respond(HttpStatusCode.InternalServerError, "Database unavailable")
            }
        }

        get("/db/user-profile/{email}") {
            val email = call.parameters["email"]
            if (email == null) {
                call.respond(HttpStatusCode.BadRequest)
                return@get
            }

            try {
                // Phase 1: Parallel Fetch
                val (user, trending) = withContext(Dispatchers.IO) {
                    val userDeferred = async {
                        dataSource.connection.use { conn ->
                            conn.prepareStatement("SELECT id, username, email, created_at, last_login, settings FROM users WHERE email = ?").use { stmt ->
                                stmt.setString(1, email)
                                stmt.executeQuery().use { rs ->
                                    if (rs.next()) {
                                        InnerUser(
                                            id = rs.getInt("id"),
                                            username = rs.getString("username"),
                                            email = rs.getString("email"),
                                            createdAt = rs.getTimestamp("created_at").toInstant().toString(),
                                            lastLogin = rs.getTimestamp("last_login")?.toInstant()?.toString(),
                                            settings = Json.parseToJsonElement(rs.getString("settings"))
                                        )
                                    } else null
                                }
                            }
                        }
                    }

                    val trendingDeferred = async {
                        dataSource.connection.use { conn ->
                            conn.prepareStatement("SELECT id, title, content, views, created_at FROM posts ORDER BY views DESC LIMIT 5").use { stmt ->
                                stmt.executeQuery().use { rs ->
                                    val list = ArrayList<PostResponse>()
                                    while (rs.next()) {
                                        list.add(PostResponse(
                                            id = rs.getInt("id"),
                                            title = rs.getString("title"),
                                            content = rs.getString("content"),
                                            views = rs.getInt("views"),
                                            createdAt = rs.getTimestamp("created_at").toInstant().toString()
                                        ))
                                    }
                                    list
                                }
                            }
                        }
                    }

                    userDeferred.await() to trendingDeferred.await()
                }

                if (user == null) {
                    call.respond(HttpStatusCode.NotFound)
                    return@get
                }

                // Phase 2: Update & Fetch Posts
                val posts = withContext(Dispatchers.IO) {
                    dataSource.connection.use { conn ->
                        // Update
                        conn.prepareStatement("UPDATE users SET last_login = NOW() WHERE id = ?").use { stmt ->
                            stmt.setInt(1, user.id)
                            stmt.executeUpdate()
                        }

                        // Fetch Posts
                        conn.prepareStatement("SELECT id, title, content, views, created_at FROM posts WHERE user_id = ? ORDER BY created_at DESC LIMIT 10").use { stmt ->
                            stmt.setInt(1, user.id)
                            stmt.executeQuery().use { rs ->
                                val list = ArrayList<PostResponse>()
                                while (rs.next()) {
                                    list.add(PostResponse(
                                        id = rs.getInt("id"),
                                        title = rs.getString("title"),
                                        content = rs.getString("content"),
                                        views = rs.getInt("views"),
                                        createdAt = rs.getTimestamp("created_at").toInstant().toString()
                                    ))
                                }
                                list
                            }
                        }
                    }
                }

                call.respond(UserProfile(
                    username = user.username,
                    email = user.email,
                    createdAt = user.createdAt,
                    lastLogin = java.time.Instant.now().toString(),
                    settings = user.settings,
                    posts = posts,
                    trending = trending
                ))

            } catch (e: Exception) {
                e.printStackTrace()
                call.respond(HttpStatusCode.InternalServerError, e.message ?: "Error")
            }
        }
    }
}

@Serializable
data class UserProfile(
    val username: String,
    val email: String,
    val createdAt: String,
    val lastLogin: String?, 
    val settings: JsonElement,
    val posts: List<PostResponse>,
    val trending: List<PostResponse>
)

@Serializable
data class PostResponse(
    val id: Int,
    val title: String,
    val content: String,
    val views: Int,
    val createdAt: String
)

data class InnerUser(
    val id: Int,
    val username: String,
    val email: String,
    val createdAt: String,
    val lastLogin: String?,
    val settings: JsonElement
)
