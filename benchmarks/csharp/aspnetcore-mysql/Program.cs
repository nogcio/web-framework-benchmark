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
var port = Environment.GetEnvironmentVariable("PORT") ?? "8000";
var dbHost = Environment.GetEnvironmentVariable("DB_HOST") ?? "db";
var dbPort = Environment.GetEnvironmentVariable("DB_PORT") ?? "3306";
var dbUser = Environment.GetEnvironmentVariable("DB_USER") ?? "benchmark";
var dbPass = Environment.GetEnvironmentVariable("DB_PASSWORD") ?? "benchmark";
var dbName = Environment.GetEnvironmentVariable("DB_NAME") ?? "benchmark";

var parsedPort = uint.TryParse(dbPort, out var portValue) ? portValue : 3306u;
var connectionStringBuilder = new MySqlConnectionStringBuilder
{
    Server = dbHost,
    Port = parsedPort,
    UserID = dbUser,
    Password = dbPass,
    Database = dbName,
    MinimumPoolSize = 256,
    MaximumPoolSize = 256,
    ConnectionReset = false,
    Pooling = true,
    SslMode = MySqlSslMode.None,
    ConnectionTimeout = 5,
    AllowPublicKeyRetrieval = true,
};

var dataSourceBuilder = new MySqlDataSourceBuilder(connectionStringBuilder.ConnectionString);
var dataSource = dataSourceBuilder.Build();
builder.Services.AddSingleton(dataSource);

var app = builder.Build();

app.Use(async (context, next) =>
{
    if (context.Request.Headers.TryGetValue("x-request-id", out var requestId))
    {
        context.Response.Headers.Append("x-request-id", requestId);
    }
    await next();
});

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

app.MapGet("/db/read/one", async (int id, HttpContext ctx, MySqlDataSource db) =>
{
    await using var conn = await db.OpenConnectionAsync(ctx.RequestAborted);
    await using var cmd = conn.CreateCommand();
    cmd.CommandText = "SELECT id, name, created_at, updated_at FROM hello_world WHERE id = @id";
    cmd.Parameters.Add(new MySqlParameter("id", MySqlDbType.Int32) { Value = id });

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

app.MapGet("/db/read/many", async (int offset, int? limit, HttpContext ctx, MySqlDataSource db) =>
{
    var actualLimit = limit ?? 50;
    await using var conn = await db.OpenConnectionAsync(ctx.RequestAborted);
    await using var cmd = conn.CreateCommand();
    cmd.CommandText = "SELECT id, name, created_at, updated_at FROM hello_world ORDER BY id LIMIT @limit OFFSET @offset";
    cmd.Parameters.Add(new MySqlParameter("limit", MySqlDbType.Int32) { Value = actualLimit });
    cmd.Parameters.Add(new MySqlParameter("offset", MySqlDbType.Int32) { Value = offset });

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

app.MapPost("/db/write/insert", async (HttpRequest request, HttpContext ctx, MySqlDataSource db) =>
{
    string? name = request.Query["name"];
    if (string.IsNullOrEmpty(name))
    {
        try
        {
            var body = await JsonSerializer.DeserializeAsync<JsonElement>(request.Body, cancellationToken: ctx.RequestAborted);
            if (body.TryGetProperty("name", out var nameProp) && nameProp.ValueKind == JsonValueKind.String)
            {
                name = nameProp.GetString();
            }
        }
        catch
        {
        }
    }

    if (string.IsNullOrEmpty(name))
    {
        return Results.BadRequest("Missing name");
    }

    var now = DateTime.UtcNow;
    await using var conn = await db.OpenConnectionAsync(ctx.RequestAborted);
    await using var insert = conn.CreateCommand();
    insert.CommandText = "INSERT INTO hello_world (name, created_at, updated_at) VALUES (@name, @created_at, @updated_at)";
    insert.Parameters.Add(new MySqlParameter("name", MySqlDbType.VarChar) { Value = name! });
    insert.Parameters.Add(new MySqlParameter("created_at", MySqlDbType.DateTime) { Value = now });
    insert.Parameters.Add(new MySqlParameter("updated_at", MySqlDbType.DateTime) { Value = now });

    await insert.ExecuteNonQueryAsync(ctx.RequestAborted);
    var insertedId = (int)insert.LastInsertedId;

    await using var select = conn.CreateCommand();
    select.CommandText = "SELECT id, name, created_at, updated_at FROM hello_world WHERE id = @id";
    select.Parameters.Add(new MySqlParameter("id", MySqlDbType.Int32) { Value = insertedId });

    await using var reader = await select.ExecuteReaderAsync(ctx.RequestAborted);
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
