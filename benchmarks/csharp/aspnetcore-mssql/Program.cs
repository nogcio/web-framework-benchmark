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

var port = Environment.GetEnvironmentVariable("PORT") ?? "8000";
var dbHost = Environment.GetEnvironmentVariable("DB_HOST") ?? "db";
var dbPort = Environment.GetEnvironmentVariable("DB_PORT") ?? "1433";
var dbUser = Environment.GetEnvironmentVariable("DB_USER") ?? "benchmark";
var dbPass = Environment.GetEnvironmentVariable("DB_PASSWORD") ?? "Benchmark!12345";
var dbName = Environment.GetEnvironmentVariable("DB_NAME") ?? "benchmark";

var connectionStringBuilder = new SqlConnectionStringBuilder
{
    DataSource = $"{dbHost},{dbPort}",
    UserID = dbUser,
    Password = dbPass,
    InitialCatalog = dbName,
    TrustServerCertificate = true,
    Encrypt = false,
    Pooling = true,
    MinPoolSize = 256,
    MaxPoolSize = 256,
    ConnectTimeout = 5,
};

var connectionString = connectionStringBuilder.ConnectionString;
builder.Services.AddSingleton(connectionString);

var app = builder.Build();

app.Use(async (context, next) =>
{
    if (context.Request.Headers.TryGetValue("x-request-id", out var requestId))
    {
        context.Response.Headers.Append("x-request-id", requestId);
    }
    await next();
});

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

app.MapGet("/db/read/one", async (int id, HttpContext ctx, [FromServices] string connectionString) =>
{
    await using var conn = new SqlConnection(connectionString);
    await conn.OpenAsync(ctx.RequestAborted);
    await using var cmd = conn.CreateCommand();
    cmd.CommandText = "SELECT TOP (1) id, name, created_at, updated_at FROM hello_world WHERE id = @id";
    cmd.Parameters.Add(new SqlParameter("@id", SqlDbType.Int) { Value = id });

    await using var reader = await cmd.ExecuteReaderAsync(ctx.RequestAborted);
    if (await reader.ReadAsync(ctx.RequestAborted))
    {
        var row = new
        {
            id = reader.GetInt32(0),
            name = reader.GetString(1),
            created_at = reader.GetDateTime(2),
            updated_at = reader.GetDateTime(3)
        };
        return Results.Json(row);
    }

    return Results.NotFound();
});

app.MapGet("/db/read/many", async (int offset, int? limit, HttpContext ctx, [FromServices] string connectionString) =>
{
    var actualLimit = limit ?? 50;
    await using var conn = new SqlConnection(connectionString);
    await conn.OpenAsync(ctx.RequestAborted);
    await using var cmd = conn.CreateCommand();
    cmd.CommandText = "SELECT id, name, created_at, updated_at FROM hello_world ORDER BY id OFFSET @offset ROWS FETCH NEXT @limit ROWS ONLY";
    cmd.Parameters.Add(new SqlParameter("@offset", SqlDbType.Int) { Value = offset });
    cmd.Parameters.Add(new SqlParameter("@limit", SqlDbType.Int) { Value = actualLimit });

    var results = new List<object>();
    await using var reader = await cmd.ExecuteReaderAsync(ctx.RequestAborted);
    while (await reader.ReadAsync(ctx.RequestAborted))
    {
        results.Add(new
        {
            id = reader.GetInt32(0),
            name = reader.GetString(1),
            created_at = reader.GetDateTime(2),
            updated_at = reader.GetDateTime(3)
        });
    }

    return Results.Json(results);
});

app.MapPost("/db/write/insert", async (HttpRequest request, HttpContext ctx, [FromServices] string connectionString) =>
{
    string? name = request.Query["name"];
    if (string.IsNullOrEmpty(name))
    {
        try
        {
            var body = await JsonSerializer.DeserializeAsync<JsonElement>(request.Body, cancellationToken: ctx.RequestAborted);
            if (body.ValueKind == JsonValueKind.Object && body.TryGetProperty("name", out var nameProp) && nameProp.ValueKind == JsonValueKind.String)
            {
                name = nameProp.GetString();
            }
        }
        catch
        {
            // Ignore malformed JSON
        }
    }

    if (string.IsNullOrEmpty(name))
    {
        return Results.BadRequest("Missing name");
    }

    var now = DateTime.SpecifyKind(DateTime.UtcNow, DateTimeKind.Unspecified);
    await using var conn = new SqlConnection(connectionString);
    await conn.OpenAsync(ctx.RequestAborted);

    await using var cmd = conn.CreateCommand();
    cmd.CommandText = @"INSERT INTO hello_world (name, created_at, updated_at)
                        OUTPUT INSERTED.id, INSERTED.name, INSERTED.created_at, INSERTED.updated_at
                        VALUES (@name, @created_at, @updated_at)";
    cmd.Parameters.Add(new SqlParameter("@name", SqlDbType.NVarChar, 255) { Value = name! });
    cmd.Parameters.Add(new SqlParameter("@created_at", SqlDbType.DateTime2) { Value = now });
    cmd.Parameters.Add(new SqlParameter("@updated_at", SqlDbType.DateTime2) { Value = now });

    await using var reader = await cmd.ExecuteReaderAsync(ctx.RequestAborted);
    if (await reader.ReadAsync(ctx.RequestAborted))
    {
        var row = new
        {
            id = reader.GetInt32(0),
            name = reader.GetString(1),
            created_at = reader.GetDateTime(2),
            updated_at = reader.GetDateTime(3)
        };
        return Results.Json(row);
    }

    return Results.StatusCode(500);
});

app.Run($"http://0.0.0.0:{port}");
