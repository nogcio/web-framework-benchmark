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
var port = Environment.GetEnvironmentVariable("PORT") ?? "8000";
var dbHost = Environment.GetEnvironmentVariable("DB_HOST") ?? "db";
var dbPort = Environment.GetEnvironmentVariable("DB_PORT") ?? "5432";
var dbUser = Environment.GetEnvironmentVariable("DB_USER") ?? "benchmark";
var dbPass = Environment.GetEnvironmentVariable("DB_PASSWORD") ?? "benchmark";
var dbName = Environment.GetEnvironmentVariable("DB_NAME") ?? "benchmark";

var connectionString = string.Join(';',
    $"Host={dbHost}",
    $"Port={dbPort}",
    $"Username={dbUser}",
    $"Password={dbPass}",
    $"Database={dbName}",
    "MaxPoolSize=256",
    "MinPoolSize=256",
    "AutoPrepareMinUsages=2",
    "MaxAutoPrepare=128"
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

app.Use(async (context, next) =>
{
    if (context.Request.Headers.TryGetValue("x-request-id", out var requestId))
    {
        context.Response.Headers.Append("x-request-id", requestId);
    }
    await next();
});

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

app.MapGet("/db/read/one", async (int id, HttpContext ctx, NpgsqlDataSource db) =>
{
    using var cmd = db.CreateCommand("SELECT id, name, created_at, updated_at FROM hello_world WHERE id = @id");
    cmd.Parameters.Add(new NpgsqlParameter<int>("id", NpgsqlDbType.Integer) { TypedValue = id });
    
    using var reader = await cmd.ExecuteReaderAsync(ctx.RequestAborted);
    if (await reader.ReadAsync(ctx.RequestAborted))
    {
        var row = new
        {
            id = reader.GetInt32(0),
            name = reader.GetString(1),
            createdAt = reader.GetDateTime(2),
            updatedAt = reader.GetDateTime(3)
        };
        return Results.Json(row);
    }
    return Results.NotFound();
});

app.MapGet("/db/read/many", async (int offset, int? limit, HttpContext ctx, NpgsqlDataSource db) =>
{
    var actualLimit = limit ?? 50;
    using var cmd = db.CreateCommand("SELECT id, name, created_at, updated_at FROM hello_world ORDER BY id LIMIT @limit OFFSET @offset");
    cmd.Parameters.Add(new NpgsqlParameter<int>("limit", NpgsqlDbType.Integer) { TypedValue = actualLimit });
    cmd.Parameters.Add(new NpgsqlParameter<int>("offset", NpgsqlDbType.Integer) { TypedValue = offset });

    var results = new List<object>();
    using var reader = await cmd.ExecuteReaderAsync(ctx.RequestAborted);
    while (await reader.ReadAsync(ctx.RequestAborted))
    {
        results.Add(new
        {
            id = reader.GetInt32(0),
            name = reader.GetString(1),
            createdAt = reader.GetDateTime(2),
            updatedAt = reader.GetDateTime(3)
        });
    }
    return Results.Json(results);
});

app.MapPost("/db/write/insert", async (HttpRequest request, HttpContext ctx, NpgsqlDataSource db) =>
{
    string? name = request.Query["name"];
    if (string.IsNullOrEmpty(name))
    {
        try
        {
            var body = await JsonSerializer.DeserializeAsync<JsonElement>(request.Body, cancellationToken: ctx.RequestAborted);
            if (body.TryGetProperty("name", out var nameProp))
            {
                name = nameProp.GetString();
            }
        }
        catch { }
    }

    if (string.IsNullOrEmpty(name))
    {
        return Results.BadRequest("Missing name");
    }

    var now = DateTime.SpecifyKind(DateTime.UtcNow, DateTimeKind.Unspecified);
    using var cmd = db.CreateCommand("INSERT INTO hello_world (name, created_at, updated_at) VALUES (@name, @created_at, @updated_at) RETURNING id, name, created_at, updated_at");
    cmd.Parameters.Add(new NpgsqlParameter<string>("name", NpgsqlDbType.Text) { TypedValue = name! });
    cmd.Parameters.Add(new NpgsqlParameter<DateTime>("created_at", NpgsqlDbType.Timestamp) { TypedValue = now });
    cmd.Parameters.Add(new NpgsqlParameter<DateTime>("updated_at", NpgsqlDbType.Timestamp) { TypedValue = now });
    using var reader = await cmd.ExecuteReaderAsync(ctx.RequestAborted);
    if (await reader.ReadAsync(ctx.RequestAborted))
    {
        var row = new
        {
            id = reader.GetInt32(0),
            name = reader.GetString(1),
            createdAt = reader.GetDateTime(2),
            updatedAt = reader.GetDateTime(3)
        };
        return Results.Json(row);
    }
    return Results.StatusCode(500);
});

app.Run($"http://0.0.0.0:{port}");