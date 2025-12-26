package com.example

import com.zaxxer.hikari.HikariConfig
import com.zaxxer.hikari.HikariDataSource
import io.ktor.http.*
import io.ktor.serialization.kotlinx.json.*
import io.ktor.server.application.*
import io.ktor.server.auth.*
import io.ktor.server.auth.jwt.*
import io.ktor.server.engine.*
import io.ktor.server.netty.*
import io.ktor.server.plugins.contentnegotiation.*
import io.ktor.server.request.*
import io.ktor.server.response.*
import io.ktor.server.routing.*
import kotlinx.serialization.Serializable
import java.sql.ResultSet
import java.time.LocalDateTime
import com.auth0.jwt.JWT
import com.auth0.jwt.algorithms.Algorithm
import java.security.MessageDigest

fun main() {
    val port = System.getenv("PORT")?.toIntOrNull() ?: 8000
    embeddedServer(Netty, port = port, host = "0.0.0.0", module = Application::module)
        .start(wait = true)
}

fun Application.module() {
    val dbHost = System.getenv("DB_HOST") ?: "localhost"
    val dbPort = System.getenv("DB_PORT") ?: "5432"
    val dbName = System.getenv("DB_NAME") ?: "benchmark"
    val dbUser = System.getenv("DB_USER") ?: "benchmark"
    val dbPass = System.getenv("DB_PASSWORD") ?: "benchmark"

    val config = HikariConfig().apply {
        jdbcUrl = "jdbc:postgresql://$dbHost:$dbPort/$dbName"
        username = dbUser
        password = dbPass
        maximumPoolSize = 256
        minimumIdle = 256
        driverClassName = "org.postgresql.Driver"
        addDataSourceProperty("cachePrepStmts", "true")
        addDataSourceProperty("prepStmtCacheSize", "250")
        addDataSourceProperty("prepStmtCacheSqlLimit", "2048")
    }
    val dataSource = HikariDataSource(config)

    install(ContentNegotiation) {
        json()
    }

    val jwtSecret = "benchmark-secret"
    val jwtIssuer = "benchmark"
    val jwtRealm = "benchmark"

    install(Authentication) {
        jwt("auth-jwt") {
            realm = jwtRealm
            verifier(
                JWT.require(Algorithm.HMAC256(jwtSecret))
                    .withIssuer(jwtIssuer)
                    .build()
            )
            validate { credential ->
                if (credential.payload.getClaim("sub").asInt() != null) {
                    JWTPrincipal(credential.payload)
                } else {
                    null
                }
            }
        }
    }

    intercept(ApplicationCallPipeline.Plugins) {
        val requestId = call.request.header("x-request-id")
        if (requestId != null) {
            call.response.header("x-request-id", requestId)
        }
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
        
        // Tweet Service
        post("/api/auth/register") {
            val creds = call.receive<AuthRequest>()
            val hash = sha256(creds.password)
            try {
                dataSource.connection.use { conn ->
                    conn.prepareStatement("INSERT INTO users (username, password_hash) VALUES (?, ?)").use { stmt ->
                        stmt.setString(1, creds.username)
                        stmt.setString(2, hash)
                        stmt.executeUpdate()
                    }
                }
                call.respond(HttpStatusCode.Created)
            } catch (e: Exception) {
                call.respond(HttpStatusCode.BadRequest)
            }
        }

        post("/api/auth/login") {
            val creds = call.receive<AuthRequest>()
            val hash = sha256(creds.password)
            dataSource.connection.use { conn ->
                conn.prepareStatement("SELECT id FROM users WHERE username = ? AND password_hash = ?").use { stmt ->
                    stmt.setString(1, creds.username)
                    stmt.setString(2, hash)
                    stmt.executeQuery().use { rs ->
                        if (rs.next()) {
                            val id = rs.getInt("id")
                            val token = JWT.create()
                                .withIssuer(jwtIssuer)
                                .withClaim("sub", id)
                                .withClaim("name", creds.username)
                                .sign(Algorithm.HMAC256(jwtSecret))
                            call.respond(mapOf("token" to token))
                        } else {
                            call.respond(HttpStatusCode.Unauthorized)
                        }
                    }
                }
            }
        }

        authenticate("auth-jwt") {
            get("/api/feed") {
                val list = ArrayList<Tweet>(20)
                dataSource.connection.use { conn ->
                    conn.prepareStatement("""
                        SELECT t.id, u.username, t.content, t.created_at, (SELECT COUNT(*) FROM likes l WHERE l.tweet_id = t.id) as likes
                        FROM tweets t
                        JOIN users u ON t.user_id = u.id
                        ORDER BY t.created_at DESC
                        LIMIT 20
                    """).use { stmt ->
                        stmt.executeQuery().use { rs ->
                            while (rs.next()) {
                                list.add(Tweet(
                                    id = rs.getInt("id"),
                                    username = rs.getString("username"),
                                    content = rs.getString("content"),
                                    createdAt = rs.getTimestamp("created_at").toInstant().toString(),
                                    likes = rs.getInt("likes")
                                ))
                            }
                        }
                    }
                }
                call.respond(list)
            }

            get("/api/tweets/{id}") {
                val id = call.parameters["id"]?.toIntOrNull() ?: return@get call.respond(HttpStatusCode.BadRequest)
                dataSource.connection.use { conn ->
                    conn.prepareStatement("""
                        SELECT t.id, u.username, t.content, t.created_at, (SELECT COUNT(*) FROM likes l WHERE l.tweet_id = t.id) as likes
                        FROM tweets t
                        JOIN users u ON t.user_id = u.id
                        WHERE t.id = ?
                    """).use { stmt ->
                        stmt.setInt(1, id)
                        stmt.executeQuery().use { rs ->
                            if (rs.next()) {
                                call.respond(Tweet(
                                    id = rs.getInt("id"),
                                    username = rs.getString("username"),
                                    content = rs.getString("content"),
                                    createdAt = rs.getTimestamp("created_at").toInstant().toString(),
                                    likes = rs.getInt("likes")
                                ))
                            } else {
                                call.respond(HttpStatusCode.NotFound)
                            }
                        }
                    }
                }
            }

            post("/api/tweets") {
                val payload = call.receive<CreateTweetRequest>()
                val principal = call.principal<JWTPrincipal>()!!
                val userId = principal.payload.getClaim("sub").asInt()
                
                dataSource.connection.use { conn ->
                    conn.prepareStatement("INSERT INTO tweets (user_id, content) VALUES (?, ?) RETURNING id").use { stmt ->
                        stmt.setInt(1, userId)
                        stmt.setString(2, payload.content)
                        stmt.executeQuery().use { rs ->
                            if (rs.next()) {
                                call.respond(HttpStatusCode.Created, mapOf("id" to rs.getInt("id")))
                            }
                        }
                    }
                }
            }

            post("/api/tweets/{id}/like") {
                val id = call.parameters["id"]?.toIntOrNull() ?: return@post call.respond(HttpStatusCode.BadRequest)
                val principal = call.principal<JWTPrincipal>()!!
                val userId = principal.payload.getClaim("sub").asInt()
                
                dataSource.connection.use { conn ->
                    conn.prepareStatement("DELETE FROM likes WHERE user_id = ? AND tweet_id = ?").use { stmt ->
                        stmt.setInt(1, userId)
                        stmt.setInt(2, id)
                        val count = stmt.executeUpdate()
                        if (count == 0) {
                            try {
                                conn.prepareStatement("INSERT INTO likes (user_id, tweet_id) VALUES (?, ?)").use { insertStmt ->
                                    insertStmt.setInt(1, userId)
                                    insertStmt.setInt(2, id)
                                    insertStmt.executeUpdate()
                                }
                            } catch (e: Exception) {
                                // Ignore
                            }
                        }
                    }
                }
                call.respond(HttpStatusCode.OK)
            }
        }
    }
}

fun sha256(input: String): String {
    val bytes = MessageDigest.getInstance("SHA-256").digest(input.toByteArray())
    return bytes.joinToString("") { "%02x".format(it) }
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
