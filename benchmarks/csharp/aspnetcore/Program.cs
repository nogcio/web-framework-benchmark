using System.Text.Json;
using System.Text.Json.Serialization;
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
var dataDir = Environment.GetEnvironmentVariable("DATA_DIR") ?? "benchmarks_data";

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

app.MapGet("/", () => "Hello, World!");

app.MapGet("/health", async (HttpContext ctx) =>
{
    return Results.Text("OK");
});

app.MapPost("/json/{from}/{to}", async (string from, string to, HttpRequest request) =>
{
    try
    {
        var payload = await JsonSerializer.DeserializeAsync<WebAppPayload>(request.Body, jsonOptions);
        if (payload == null) return Results.BadRequest();

        ReplaceServletNames(payload, from, to);
        return Results.Json(payload, jsonOptions);
    }
    catch
    {
        return Results.BadRequest();
    }
});

void ReplaceServletNames(WebAppPayload payload, string from, string to)
{
    var servlets = payload.WebApp?.Servlets;
    if (servlets == null) return;

    foreach (var servlet in servlets)
    {
        if (string.Equals(servlet.ServletName, from, StringComparison.Ordinal))
        {
            servlet.ServletName = to;
        }
    }
}

app.MapGet("/files/{filename}", async (string filename, HttpContext context) =>
{
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

class WebAppPayload
{
    [JsonPropertyName("web-app")]
    public WebAppSection? WebApp { get; set; }

    [JsonExtensionData]
    public Dictionary<string, JsonElement>? Extra { get; set; }
}

class WebAppSection
{
    [JsonPropertyName("servlet")]
    public List<ServletConfig>? Servlets { get; set; }

    [JsonExtensionData]
    public Dictionary<string, JsonElement>? Extra { get; set; }
}

class ServletConfig
{
    [JsonPropertyName("servlet-name")]
    public string? ServletName { get; set; }

    [JsonExtensionData]
    public Dictionary<string, JsonElement>? Extra { get; set; }
}
