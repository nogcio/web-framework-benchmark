using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.Extensions.FileProviders;
using Microsoft.Extensions.Logging;

ThreadPool.SetMinThreads(Environment.ProcessorCount * 32, Environment.ProcessorCount * 32);

var builder = WebApplication.CreateBuilder(args);
builder.WebHost.ConfigureKestrel(options =>
{
    options.Limits.MaxConcurrentConnections = null;
    options.Limits.MaxConcurrentUpgradedConnections = null;
});
builder.Logging.ClearProviders();
var port = Environment.GetEnvironmentVariable("PORT") ?? "8080";
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

if (Directory.Exists(dataDir))
{
    app.UseStaticFiles(new StaticFileOptions
    {
        FileProvider = new PhysicalFileProvider(Path.GetFullPath(dataDir)),
        RequestPath = "/files",
        ServeUnknownFileTypes = true,
        DefaultContentType = "application/octet-stream"
    });
}

app.MapGet("/", () => "Hello, World!");
app.MapGet("/plaintext", () => "Hello, World!");

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
