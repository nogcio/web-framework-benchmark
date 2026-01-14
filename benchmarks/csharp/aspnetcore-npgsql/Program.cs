using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.Extensions.Logging;
using NpgsqlTypes;
using Npgsql;

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
var dbPort = Environment.GetEnvironmentVariable("DB_PORT") ?? "5432";
var dbUser = Environment.GetEnvironmentVariable("DB_USER") ?? "benchmark";
var dbPass = Environment.GetEnvironmentVariable("DB_PASSWORD") ?? "benchmark";
var dbName = Environment.GetEnvironmentVariable("DB_NAME") ?? "benchmark";
var dbPoolSize = Environment.GetEnvironmentVariable("DB_POOL_SIZE") ?? "256";

var connectionString = string.Join(';',
    $"Host={dbHost}",
    $"Port={dbPort}",
    $"Username={dbUser}",
    $"Password={dbPass}",
    $"Database={dbName}",
    $"MaxPoolSize={dbPoolSize}",
    $"MinPoolSize={dbPoolSize}",
    "AutoPrepareMinUsages=2",
    "MaxAutoPrepare=128",
    "SSL Mode=Disable"
);

var dataSourceBuilder = new NpgsqlDataSourceBuilder(connectionString);
var dataSource = dataSourceBuilder.Build();

builder.Services.AddSingleton(dataSource);

var app = builder.Build();

var jsonOptions = new JsonSerializerOptions
{
    PropertyNameCaseInsensitive = false,
    DefaultIgnoreCondition = JsonIgnoreCondition.Never
};

app.MapGet("/health", async (HttpContext ctx, NpgsqlDataSource db) =>
{
    try
    {
        using var cmd = db.CreateCommand("SELECT 1");
        await cmd.ExecuteScalarAsync(ctx.RequestAborted);
        return Results.Text("OK");
    }
    catch
    {
        return Results.Text("Service Unavailable", statusCode: 503);
    }
});

app.MapGet("/db/user-profile/{email}", async (string email, HttpContext ctx, NpgsqlDataSource db) =>
{
    // Parallel Phase 1: Get User & Trending
    var userTask = GetUserAsync(db, email, ctx.RequestAborted);
    var trendingTask = GetTrendingAsync(db, ctx.RequestAborted);

    await Task.WhenAll(userTask, trendingTask);

    var user = await userTask;
    var trending = await trendingTask;

    if (user == null) return Results.NotFound();

    var now = DateTime.UtcNow;

    // Parallel Phase 2: Update Last Login & Get Posts
    var updateTask = UpdateLastLoginAsync(db, user.Id, now, ctx.RequestAborted);
    var postsTask = GetPostsAsync(db, user.Id, ctx.RequestAborted);

    await Task.WhenAll(updateTask, postsTask);
    
    var posts = await postsTask;

    return Results.Ok(new
    {
        username = user.Username,
        email = user.Email,
        createdAt = user.CreatedAt.ToString("yyyy-MM-ddTHH:mm:ssZ"),
        lastLogin = now.ToString("yyyy-MM-ddTHH:mm:ssZ"), 
        settings = JsonSerializer.Deserialize<JsonElement>(user.SettingsJson),
        posts,
        trending
    });
});

async Task<User?> GetUserAsync(NpgsqlDataSource db, string email, CancellationToken ct)
{
    await using var cmd = db.CreateCommand("SELECT id, username, email, created_at, last_login, settings FROM users WHERE email = @email");
    cmd.Parameters.Add(new NpgsqlParameter<string>("email", NpgsqlDbType.Text) { TypedValue = email });

    await using var reader = await cmd.ExecuteReaderAsync(ct);
    if (await reader.ReadAsync(ct))
    {
        return new User(
            reader.GetInt32(0),
            reader.GetString(1),
            reader.GetString(2),
            reader.GetDateTime(3),
            reader.IsDBNull(4) ? null : reader.GetDateTime(4),
            reader.GetString(5)
        );
    }
    return null;
}

async Task<List<object>> GetTrendingAsync(NpgsqlDataSource db, CancellationToken ct)
{
    await using var cmd = db.CreateCommand("SELECT id, title, content, views, created_at FROM posts ORDER BY views DESC LIMIT 5");
    var list = new List<object>();
    await using var reader = await cmd.ExecuteReaderAsync(ct);
    while (await reader.ReadAsync(ct))
    {
        list.Add(new
        {
            id = reader.GetInt32(0),
            title = reader.GetString(1),
            content = reader.GetString(2),
            views = reader.GetInt32(3),
            createdAt = reader.GetDateTime(4).ToString("yyyy-MM-ddTHH:mm:ssZ")
        });
    }
    return list;
}

async Task UpdateLastLoginAsync(NpgsqlDataSource db, int userId, DateTime timestamp, CancellationToken ct)
{
    await using var cmd = db.CreateCommand("UPDATE users SET last_login = @timestamp WHERE id = @id");
    cmd.Parameters.Add(new NpgsqlParameter<int>("id", NpgsqlDbType.Integer) { TypedValue = userId });
    cmd.Parameters.Add(new NpgsqlParameter<DateTime>("timestamp", NpgsqlDbType.TimestampTz) { TypedValue = timestamp });
    await cmd.ExecuteNonQueryAsync(ct);
}

async Task<List<object>> GetPostsAsync(NpgsqlDataSource db, int userId, CancellationToken ct)
{
    await using var cmd = db.CreateCommand("SELECT id, title, content, views, created_at FROM posts WHERE user_id = @id ORDER BY created_at DESC LIMIT 10");
    cmd.Parameters.Add(new NpgsqlParameter<int>("id", NpgsqlDbType.Integer) { TypedValue = userId });

    var list = new List<object>();
    await using var reader = await cmd.ExecuteReaderAsync(ct);
    while (await reader.ReadAsync(ct))
    {
        list.Add(new
        {
            id = reader.GetInt32(0),
            title = reader.GetString(1),
            content = reader.GetString(2),
            views = reader.GetInt32(3),
            createdAt = reader.GetDateTime(4).ToString("yyyy-MM-ddTHH:mm:ssZ")
        });
    }
    return list;
}

app.Run($"http://0.0.0.0:{port}");

record User(int Id, string Username, string Email, DateTime CreatedAt, DateTime? LastLogin, string SettingsJson);
