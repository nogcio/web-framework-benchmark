using System.Text.Json;
using System.Text.Json.Nodes;
using Npgsql;

var builder = WebApplication.CreateBuilder(args);

// Configuration
var port = Environment.GetEnvironmentVariable("PORT") ?? "8000";
var dbHost = Environment.GetEnvironmentVariable("DB_HOST") ?? "db";
var dbPort = Environment.GetEnvironmentVariable("DB_PORT") ?? "5432";
var dbUser = Environment.GetEnvironmentVariable("DB_USER") ?? "benchmark";
var dbPass = Environment.GetEnvironmentVariable("DB_PASSWORD") ?? "benchmark";
var dbName = Environment.GetEnvironmentVariable("DB_NAME") ?? "benchmark";
var dataDir = Environment.GetEnvironmentVariable("DATA_DIR") ?? "benchmarks_data";

var connectionString = $"Host={dbHost};Port={dbPort};Username={dbUser};Password={dbPass};Database={dbName}";

var dataSourceBuilder = new NpgsqlDataSourceBuilder(connectionString);
var dataSource = dataSourceBuilder.Build();

builder.Services.AddSingleton(dataSource);

var app = builder.Build();

// Middleware for x-request-id
app.Use(async (context, next) =>
{
    if (context.Request.Headers.TryGetValue("x-request-id", out var requestId))
    {
        context.Response.Headers.Append("x-request-id", requestId);
    }
    await next();
});

// 5.1. Root / Hello World
app.MapGet("/", () => "Hello, World!");

// 5.2. Health Check
app.MapGet("/health", async (NpgsqlDataSource db) =>
{
    try
    {
        using var cmd = db.CreateCommand("SELECT 1");
        await cmd.ExecuteScalarAsync();
        return Results.Text("OK");
    }
    catch
    {
        return Results.Text("Service Unavailable", statusCode: 503);
    }
});

// 5.3. Info
app.MapGet("/info", () => "10.0,hello_world,json,db_read_one,db_read_paging,db_write,static_files");

// 5.4. JSON Processing
app.MapPost("/json/{from}/{to}", async (string from, string to, HttpRequest request) =>
{
    try
    {
        var node = await JsonSerializer.DeserializeAsync<JsonNode>(request.Body);
        if (node == null) return Results.BadRequest();

        Traverse(node, from, to);
        return Results.Json(node);
    }
    catch
    {
        return Results.BadRequest();
    }
});

void Traverse(JsonNode node, string from, string to)
{
    if (node is JsonObject obj)
    {
        if (obj.TryGetPropertyValue("servlet-name", out var val) && val?.GetValue<string>() == from)
        {
            obj["servlet-name"] = to;
        }

        foreach (var property in obj.ToList()) // ToList to avoid modification during iteration issues if any, though here we modify values not keys
        {
            if (property.Value != null)
                Traverse(property.Value, from, to);
        }
    }
    else if (node is JsonArray arr)
    {
        foreach (var item in arr)
        {
            if (item != null)
                Traverse(item, from, to);
        }
    }
}

// 5.5. Database: Read One
app.MapGet("/db/read/one", async (int id, NpgsqlDataSource db) =>
{
    using var cmd = db.CreateCommand("SELECT id, name, created_at, updated_at FROM hello_world WHERE id = $1");
    cmd.Parameters.AddWithValue(id);
    
    using var reader = await cmd.ExecuteReaderAsync();
    if (await reader.ReadAsync())
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

// 5.6. Database: Read Many (Paging)
app.MapGet("/db/read/many", async (int offset, int? limit, NpgsqlDataSource db) =>
{
    var actualLimit = limit ?? 50;
    using var cmd = db.CreateCommand("SELECT id, name, created_at, updated_at FROM hello_world ORDER BY id LIMIT $1 OFFSET $2");
    cmd.Parameters.AddWithValue(actualLimit);
    cmd.Parameters.AddWithValue(offset);

    var results = new List<object>();
    using var reader = await cmd.ExecuteReaderAsync();
    while (await reader.ReadAsync())
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

// 5.7. Database: Write (Insert)
app.MapPost("/db/write/insert", async (HttpRequest request, NpgsqlDataSource db) =>
{
    string? name = request.Query["name"];
    if (string.IsNullOrEmpty(name))
    {
        try
        {
            var body = await JsonSerializer.DeserializeAsync<JsonElement>(request.Body);
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

    using var cmd = db.CreateCommand("INSERT INTO hello_world (name, created_at, updated_at) VALUES ($1, NOW(), NOW()) RETURNING id, name, created_at, updated_at");
    cmd.Parameters.AddWithValue(name);

    using var reader = await cmd.ExecuteReaderAsync();
    if (await reader.ReadAsync())
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

// 5.8. Static Files
app.MapGet("/files/{filename}", async (string filename, HttpContext context) =>
{
    // Security check
    if (filename.Contains("..") || filename.Contains('/') || filename.Contains('\\'))
    {
        return Results.Forbid();
    }

    var filePath = Path.Combine(dataDir, filename);
    if (!File.Exists(filePath))
    {
        return Results.NotFound();
    }

    context.Response.ContentType = "application/octet-stream";
    await context.Response.SendFileAsync(filePath);
    return Results.Empty;
});

app.Run($"http://0.0.0.0:{port}");
