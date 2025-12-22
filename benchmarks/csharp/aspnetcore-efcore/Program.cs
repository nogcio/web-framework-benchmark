using System.Text.Json;
using Microsoft.Data.SqlClient;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;
using MySql.Data.MySqlClient;
using MySql.EntityFrameworkCore.Extensions;
using Npgsql;

var builder = WebApplication.CreateBuilder(args);
builder.Logging.ClearProviders();
builder.Logging.AddSimpleConsole();
builder.Logging.SetMinimumLevel(LogLevel.Error);

var port = Environment.GetEnvironmentVariable("PORT") ?? "8000";
var dbHost = Environment.GetEnvironmentVariable("DB_HOST") ?? "db";
var dbPortEnv = Environment.GetEnvironmentVariable("DB_PORT") ?? "5432";
var dbUser = Environment.GetEnvironmentVariable("DB_USER") ?? "benchmark";
var dbPass = Environment.GetEnvironmentVariable("DB_PASSWORD") ?? "benchmark";
var dbName = Environment.GetEnvironmentVariable("DB_NAME") ?? "benchmark";
var dbKindEnv = Environment.GetEnvironmentVariable("DB_KIND") ?? "postgres";
var dbKind = dbKindEnv.ToLowerInvariant();
var isMysql = dbKind == "mysql";
var isMssql = dbKind == "mssql" || dbKind == "sqlserver";

var dbPort = ushort.TryParse(dbPortEnv, out var parsedPort)
    ? parsedPort
    : (ushort)(isMysql ? 3306 : isMssql ? 1433 : 5432);

if (isMysql)
{
    var csb = new MySqlConnectionStringBuilder
    {
        Server = dbHost,
        Port = dbPort,
        UserID = dbUser,
        Password = dbPass,
        Database = dbName,
        MinimumPoolSize = 256,
        MaximumPoolSize = 256,
        ConnectionReset = false,
        Pooling = true,
        // Prefer TLS when available but allow non-TLS (matches connector defaults)
        SslMode = MySqlSslMode.Preferred,
    };

    builder.Services.AddDbContextPool<BenchmarkContext>(options =>
    {
        options.UseMySQL(csb.ConnectionString, my => my.EnableRetryOnFailure());
    });
}
else if (isMssql)
{
    var csb = new SqlConnectionStringBuilder
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

    builder.Services.AddDbContextPool<BenchmarkContext>(options =>
    {
        options.UseSqlServer(csb.ConnectionString, sql => sql.EnableRetryOnFailure());
    });
}
else
{
    var csb = new NpgsqlConnectionStringBuilder
    {
        Host = dbHost,
        Port = dbPort,
        Username = dbUser,
        Password = dbPass,
        Database = dbName,
        MaxPoolSize = 256,
        MinPoolSize = 256,
        AutoPrepareMinUsages = 2,
        MaxAutoPrepare = 128,
    };

    builder.Services.AddDbContextPool<BenchmarkContext>(options =>
    {
        options.UseNpgsql(csb.ConnectionString, npgsql => npgsql.EnableRetryOnFailure());
    });
}

var app = builder.Build();

app.Use(async (context, next) =>
{
    if (context.Request.Headers.TryGetValue("x-request-id", out var requestId))
    {
        context.Response.Headers.Append("x-request-id", requestId);
    }
    await next();
});

app.MapGet("/health", async (BenchmarkContext db) =>
{
    var canConnect = await db.Database.CanConnectAsync();
    return canConnect ? Results.Text("OK") : Results.Text("Service Unavailable", statusCode: 503);
});

app.MapGet("/db/read/one", async (int id, BenchmarkContext db, CancellationToken ct) =>
{
    var row = await db.HelloWorld
        .AsNoTracking()
        .Where(r => r.Id == id)
        .Select(r => new { r.Id, r.Name, r.CreatedAt, r.UpdatedAt })
        .FirstOrDefaultAsync(ct);

    return row is null ? Results.NotFound() : Results.Json(row);
});

app.MapGet("/db/read/many", async (int offset, int? limit, BenchmarkContext db, CancellationToken ct) =>
{
    var actualLimit = limit ?? 50;
    var rows = await db.HelloWorld
        .AsNoTracking()
        .OrderBy(r => r.Id)
        .Skip(offset)
        .Take(actualLimit)
        .Select(r => new { r.Id, r.Name, r.CreatedAt, r.UpdatedAt })
        .ToListAsync(ct);

    return Results.Json(rows);
});

app.MapPost("/db/write/insert", async (HttpRequest request, BenchmarkContext db, CancellationToken ct) =>
{
    string? name = request.Query["name"];
    if (string.IsNullOrEmpty(name))
    {
        try
        {
            var body = await JsonSerializer.DeserializeAsync<JsonElement>(request.Body, cancellationToken: ct);
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

    var now = DateTime.UtcNow;
    var entity = new HelloWorld
    {
        Name = name!,
        CreatedAt = now,
        UpdatedAt = now,
    };

    db.HelloWorld.Add(entity);
    await db.SaveChangesAsync(ct);

    var result = new { entity.Id, entity.Name, entity.CreatedAt, entity.UpdatedAt };
    return Results.Json(result);
});

app.Run($"http://0.0.0.0:{port}");

public class HelloWorld
{
    public int Id { get; set; }
    public string Name { get; set; } = string.Empty;
    public DateTime CreatedAt { get; set; }
    public DateTime UpdatedAt { get; set; }
}

public class BenchmarkContext : DbContext
{
    public BenchmarkContext(DbContextOptions<BenchmarkContext> options) : base(options) { }

    public DbSet<HelloWorld> HelloWorld => Set<HelloWorld>();

    protected override void OnModelCreating(ModelBuilder modelBuilder)
    {
        var entity = modelBuilder.Entity<HelloWorld>();
        entity.ToTable("hello_world");
        entity.HasKey(e => e.Id);
        entity.Property(e => e.Id).HasColumnName("id");
        entity.Property(e => e.Name).HasColumnName("name");
        entity.Property(e => e.CreatedAt).HasColumnName("created_at");
        entity.Property(e => e.UpdatedAt).HasColumnName("updated_at");
    }
}
