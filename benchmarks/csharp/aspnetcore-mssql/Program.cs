using System.Data;
using System.Text.Json;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Data.SqlClient;
using Microsoft.Extensions.Logging;

ThreadPool.SetMinThreads(Environment.ProcessorCount * 32, Environment.ProcessorCount * 32);

var builder = WebApplication.CreateBuilder(args);
builder.WebHost.ConfigureKestrel(options =>
{
    options.Limits.MaxConcurrentConnections = null;
    options.Limits.MaxConcurrentUpgradedConnections = null;
});
builder.Logging.ClearProviders();
builder.Logging.AddSimpleConsole();
builder.Logging.SetMinimumLevel(LogLevel.Error);

var port = Environment.GetEnvironmentVariable("PORT") ?? "8080";
var dbHost = Environment.GetEnvironmentVariable("DB_HOST") ?? "db";
var dbPort = Environment.GetEnvironmentVariable("DB_PORT") ?? "1433";
var dbUser = Environment.GetEnvironmentVariable("DB_USER") ?? "benchmark";
var dbPass = Environment.GetEnvironmentVariable("DB_PASSWORD") ?? "Benchmark!12345";
var dbName = Environment.GetEnvironmentVariable("DB_NAME") ?? "benchmark";
var dbPoolSize = int.Parse(Environment.GetEnvironmentVariable("DB_POOL_SIZE") ?? "256");

var connectionStringBuilder = new SqlConnectionStringBuilder
{
    DataSource = $"{dbHost},{dbPort}",
    UserID = dbUser,
    Password = dbPass,
    InitialCatalog = dbName,
    TrustServerCertificate = true,
    Encrypt = false,
    Pooling = true,
    MinPoolSize = dbPoolSize,
    MaxPoolSize = dbPoolSize,
    ConnectTimeout = 5,
};

var connectionString = connectionStringBuilder.ConnectionString;
builder.Services.AddSingleton(connectionString);

var app = builder.Build();

app.MapGet("/health", async (HttpContext ctx, [FromServices] string connectionString) =>
{
    try
    {
        await using var conn = new SqlConnection(connectionString);
        await conn.OpenAsync(ctx.RequestAborted);
        await using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT 1";
        await cmd.ExecuteScalarAsync(ctx.RequestAborted);
        return Results.Text("OK");
    }
    catch (Exception ex)
    {
        Console.Error.WriteLine($"health db error: {ex}");
        return Results.Text("Service Unavailable", statusCode: 503);
    }
});

app.MapGet("/db/user-profile/{email}", async (string email, HttpContext ctx, [FromServices] string connectionString) =>
{
    await using var conn = new SqlConnection(connectionString);
    await conn.OpenAsync(ctx.RequestAborted);

    // Step 1: Get User + Trending Posts (Batch)
    await using var cmd1 = conn.CreateCommand();
    cmd1.CommandText = @"
        SELECT id, username, email, created_at, last_login, settings FROM users WHERE email = @email;
        SELECT TOP 5 id, title, content, views, created_at FROM posts ORDER BY views DESC;
    ";
    cmd1.Parameters.Add(new SqlParameter("@email", SqlDbType.NVarChar, 255) { Value = email });

    int userId = 0;
    string username = "";
    DateTime createdAt = default;
    string settingsJson = "";
    var trending = new List<object>();

    await using (var reader = await cmd1.ExecuteReaderAsync(ctx.RequestAborted))
    {
        if (await reader.ReadAsync(ctx.RequestAborted))
        {
            userId = reader.GetInt32(0);
            username = reader.GetString(1);
            // email is arg
            createdAt = reader.GetDateTime(3);
            // last_login (4) ignored for read, we update it later
            settingsJson = reader.GetString(5);
        }
        else
        {
            return Results.NotFound();
        }

        await reader.NextResultAsync(ctx.RequestAborted);

        while (await reader.ReadAsync(ctx.RequestAborted))
        {
            trending.Add(new
            {
                id = reader.GetInt32(0),
                title = reader.GetString(1),
                content = reader.GetString(2),
                views = reader.GetInt32(3),
                createdAt = reader.GetDateTime(4).ToString("yyyy-MM-ddTHH:mm:ssZ")
            });
        }
    }

    // Step 2: Update Last Login + Get User Posts
    await using var cmd2 = conn.CreateCommand();
    cmd2.CommandText = @"
        UPDATE users SET last_login = SYSDATETIME() WHERE id = @id;
        SELECT TOP 10 id, title, content, views, created_at FROM posts WHERE user_id = @id ORDER BY created_at DESC;
    ";
    cmd2.Parameters.Add(new SqlParameter("@id", SqlDbType.Int) { Value = userId });

    var posts = new List<object>();
    await using (var reader = await cmd2.ExecuteReaderAsync(ctx.RequestAborted))
    {
        // UPDATE produces no result set, so the first result set is from SELECT
        while (await reader.ReadAsync(ctx.RequestAborted))
        {
            posts.Add(new
            {
                id = reader.GetInt32(0),
                title = reader.GetString(1),
                content = reader.GetString(2),
                views = reader.GetInt32(3),
                createdAt = reader.GetDateTime(4).ToString("yyyy-MM-ddTHH:mm:ssZ")
            });
        }
    }

    object settingsObj;
    try { settingsObj = JsonSerializer.Deserialize<JsonElement>(settingsJson); } catch { settingsObj = settingsJson; }

    return Results.Ok(new
    {
        username,
        email,
        createdAt = createdAt.ToString("yyyy-MM-ddTHH:mm:ssZ"),
        lastLogin = DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ssZ"),
        settings = settingsObj,
        posts,
        trending
    });
});

app.Run($"http://0.0.0.0:{port}");
