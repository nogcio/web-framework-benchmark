using System.Text.Json;
using Microsoft.Extensions.Logging;
using MySqlConnector;

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
var dbPort = Environment.GetEnvironmentVariable("DB_PORT") ?? "3306";
var dbUser = Environment.GetEnvironmentVariable("DB_USER") ?? "benchmark";
var dbPass = Environment.GetEnvironmentVariable("DB_PASSWORD") ?? "benchmark";
var dbName = Environment.GetEnvironmentVariable("DB_NAME") ?? "benchmark";
var dbPoolSize = uint.Parse(Environment.GetEnvironmentVariable("DB_POOL_SIZE") ?? "256");

var parsedPort = uint.TryParse(dbPort, out var portValue) ? portValue : 3306u;
var connectionStringBuilder = new MySqlConnectionStringBuilder
{
    Server = dbHost,
    Port = parsedPort,
    UserID = dbUser,
    Password = dbPass,
    Database = dbName,
    MinimumPoolSize = dbPoolSize,
    MaximumPoolSize = dbPoolSize,
    ConnectionReset = false,
    Pooling = true,
    SslMode = MySqlSslMode.None,
    ConnectionTimeout = 30,
    AllowPublicKeyRetrieval = true,
};

var dataSourceBuilder = new MySqlDataSourceBuilder(connectionStringBuilder.ConnectionString);
var dataSource = dataSourceBuilder.Build();
builder.Services.AddSingleton(dataSource);

var app = builder.Build();

app.MapGet("/health", async (HttpContext ctx, MySqlDataSource db) =>
{
    try
    {
        await using var conn = await db.OpenConnectionAsync(ctx.RequestAborted);
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

app.MapGet("/db/user-profile/{email}", async (string email, HttpContext ctx, MySqlDataSource db) =>
{
    await using var conn = await db.OpenConnectionAsync(ctx.RequestAborted);

    // Step 1: Get User + Trending Posts (Batch)
    await using var cmd1 = conn.CreateCommand();
    cmd1.CommandText = @"
        SELECT id, username, email, created_at, last_login, settings FROM users WHERE email = @email;
        SELECT id, title, content, views, created_at FROM posts ORDER BY views DESC LIMIT 5;
    ";
    cmd1.Parameters.Add(new MySqlParameter("email", MySqlDbType.VarChar) { Value = email });

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
            createdAt = reader.GetDateTime(3);
            // last_login (4) ignored
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
        UPDATE users SET last_login = NOW() WHERE id = @id;
        SELECT id, title, content, views, created_at FROM posts WHERE user_id = @id ORDER BY created_at DESC LIMIT 10;
    ";
    cmd2.Parameters.Add(new MySqlParameter("id", MySqlDbType.Int32) { Value = userId });

    var posts = new List<object>();
    await using (var reader = await cmd2.ExecuteReaderAsync(ctx.RequestAborted))
    {
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
