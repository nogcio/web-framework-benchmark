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
    PropertyNameCaseInsensitive = true,
    DefaultIgnoreCondition = JsonIgnoreCondition.Never
};

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

app.MapPost("/json/aggregate", async (HttpRequest request) =>
{
    try
    {
        var orders = await JsonSerializer.DeserializeAsync<List<Order>>(request.Body, jsonOptions);
        if (orders == null) return Results.BadRequest();

        var processedOrders = 0;
        var results = new Dictionary<string, long>();
        var categoryStats = new Dictionary<string, int>();

        foreach (var order in orders)
        {
            if (order.Status == "completed")
            {
                processedOrders++;

                if (!results.ContainsKey(order.Country))
                    results[order.Country] = 0;
                results[order.Country] += order.Amount;

                if (order.Items != null)
                {
                    foreach (var item in order.Items)
                    {
                        if (!categoryStats.ContainsKey(item.Category))
                            categoryStats[item.Category] = 0;
                        categoryStats[item.Category] += item.Quantity;
                    }
                }
            }
        }

        return Results.Json(new AggregateResponse
        {
            ProcessedOrders = processedOrders,
            Results = results,
            CategoryStats = categoryStats
        }, jsonOptions);
    }
    catch
    {
        return Results.BadRequest();
    }
});

app.Run($"http://0.0.0.0:{port}");

class Order
{
    public string Status { get; set; } = "";
    public long Amount { get; set; }
    public string Country { get; set; } = "";
    public List<OrderItem>? Items { get; set; }
}

class OrderItem
{
    public int Quantity { get; set; }
    public string Category { get; set; } = "";
}

class AggregateResponse
{
    [JsonPropertyName("processedOrders")]
    public int ProcessedOrders { get; set; }

    [JsonPropertyName("results")]
    public Dictionary<string, long> Results { get; set; } = new();

    [JsonPropertyName("categoryStats")]
    public Dictionary<string, int> CategoryStats { get; set; } = new();
}