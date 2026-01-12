using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;
using System.Security.Claims;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.EntityFrameworkCore;
using Microsoft.Data.SqlClient;
using MySqlConnector;
using Npgsql;

var builder = WebApplication.CreateBuilder(args);
builder.WebHost.ConfigureKestrel(options =>
{
    options.Limits.MaxConcurrentConnections = null;
    options.Limits.MaxConcurrentUpgradedConnections = null;
});
builder.Logging.ClearProviders();
builder.Logging.AddSimpleConsole();
builder.Logging.SetMinimumLevel(LogLevel.Error);

// --- Database Configuration ---
var port = Environment.GetEnvironmentVariable("PORT") ?? "8080";
var dbHost = Environment.GetEnvironmentVariable("DB_HOST") ?? "db";
var dbPortEnv = Environment.GetEnvironmentVariable("DB_PORT");
var dbUser = Environment.GetEnvironmentVariable("DB_USER") ?? "benchmark";
var dbPass = Environment.GetEnvironmentVariable("DB_PASSWORD") ?? "benchmark";
var dbName = Environment.GetEnvironmentVariable("DB_NAME") ?? "benchmark";
var dbKindEnv = Environment.GetEnvironmentVariable("DB_KIND");

var dbKind = dbKindEnv.ToLowerInvariant();
var isMysql = dbKind == "mysql" || dbKind == "mariadb";
BenchmarkContext.IsMySql = isMysql;
var isMssql = dbKind == "mssql" || dbKind == "sqlserver";
var dbPoolSize = int.Parse(Environment.GetEnvironmentVariable("DB_POOL_SIZE") ?? "256");

var dbPortStr = dbPortEnv ?? (isMysql ? "3306" : isMssql ? "1433" : "5432");
var dbPort = ushort.Parse(dbPortStr);

if (isMysql)
{
    var csb = new MySqlConnectionStringBuilder
    {
        Server = dbHost,
        Port = dbPort,
        UserID = dbUser,
        Password = dbPass,
        Database = dbName,
        MinimumPoolSize = (uint)dbPoolSize,
        MaximumPoolSize = (uint)dbPoolSize,
        ConnectionReset = false,
        Pooling = true,
        SslMode = MySqlSslMode.None,
    };
    builder.Services.AddDbContextPool<BenchmarkContext>(options =>
        options.UseMySql(csb.ConnectionString, ServerVersion.AutoDetect(csb.ConnectionString), my => my.EnableRetryOnFailure()));
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
        MinPoolSize = dbPoolSize,
        MaxPoolSize = dbPoolSize,
        ConnectTimeout = 5,
    };
    builder.Services.AddDbContextPool<BenchmarkContext>(options =>
        options.UseSqlServer(csb.ConnectionString, sql => sql.EnableRetryOnFailure()));
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
        MaxPoolSize = dbPoolSize,
        MinPoolSize = dbPoolSize,
        AutoPrepareMinUsages = 2,
        MaxAutoPrepare = 128,
    };
    builder.Services.AddDbContextPool<BenchmarkContext>(options =>
        options.UseNpgsql(csb.ConnectionString, npgsql => npgsql.EnableRetryOnFailure()));
}

var app = builder.Build();

app.MapGet("/health", async (BenchmarkContext db) =>
{
    var canConnect = await db.Database.CanConnectAsync();
    return canConnect ? Results.Text("OK") : Results.Text("Service Unavailable", statusCode: 503);
});

async Task<IResult> GetUserProfile(string email, BenchmarkContext db, ILogger logger)
{
    // Sequential execution
    var user = await db.Users.FirstOrDefaultAsync(u => u.Email == email);
    if (user == null) return Results.NotFound();

    // Update LastLogin
    user.LastLogin = DateTime.UtcNow;
    await db.SaveChangesAsync();

    var trending = await db.Posts.AsNoTracking().OrderByDescending(p => p.Views).Take(5).ToListAsync();
    var posts = await db.Posts.AsNoTracking().Where(p => p.UserId == user.Id).OrderByDescending(p => p.CreatedAt).Take(10).ToListAsync();

    object settings;
    try
    {
        settings = JsonSerializer.Deserialize<JsonElement>(user.Settings);
    }
    catch (Exception ex)
    {
        logger.LogError(ex, "Failed to deserialize settings for user {Email}. Settings value: {Settings}", user.Email, user.Settings);
        settings = user.Settings;
    }

    return Results.Ok(new
    {
        username = user.Username,
        email = user.Email,
        createdAt = user.CreatedAt.ToString("yyyy-MM-ddTHH:mm:ssZ"),
        lastLogin = user.LastLogin?.ToString("yyyy-MM-ddTHH:mm:ssZ"),
        settings = settings,
        posts = posts.Select(p => new
        {
            id = p.Id,
            title = p.Title,
            content = p.Content,
            views = p.Views,
            createdAt = p.CreatedAt.ToString("yyyy-MM-ddTHH:mm:ssZ")
        }),
        trending = trending.Select(p => new
        {
            id = p.Id,
            title = p.Title,
            content = p.Content,
            views = p.Views,
            createdAt = p.CreatedAt.ToString("yyyy-MM-ddTHH:mm:ssZ")
        })
    });
}

app.MapGet("/db/user-profile/{email}", async (string email, BenchmarkContext db, ILogger<Program> logger) =>
{
    try
    {
        return await GetUserProfile(email, db, logger);
    }
    catch (Exception ex)
    {
        logger.LogError(ex, "Unhandled exception for {Email}", email);
        return Results.StatusCode(500);
    }
});

app.Run($"http://0.0.0.0:{port}");

public class BenchmarkContext : DbContext
{
    public static bool IsMySql { get; set; }

    public BenchmarkContext(DbContextOptions<BenchmarkContext> options) : base(options) { }

    public DbSet<User> Users => Set<User>();
    public DbSet<Post> Posts => Set<Post>();

    protected override void OnModelCreating(ModelBuilder modelBuilder)
    {
        var user = modelBuilder.Entity<User>();
        user.ToTable("users");
        user.Property(u => u.Id).HasColumnName("id");
        user.Property(u => u.Username).HasColumnName("username");
        user.Property(u => u.Email).HasColumnName("email");
        user.Property(u => u.CreatedAt).HasColumnName("created_at");
        user.Property(u => u.LastLogin).HasColumnName("last_login");
        user.Property(u => u.Settings).HasColumnName("settings");
        if (IsMySql)
        {
            user.Property(u => u.Settings).HasColumnType("longtext");
        }

        var post = modelBuilder.Entity<Post>();
        post.ToTable("posts");
        post.Property(p => p.Id).HasColumnName("id");
        post.Property(p => p.UserId).HasColumnName("user_id");
        post.Property(p => p.Title).HasColumnName("title");
        post.Property(p => p.Content).HasColumnName("content");
        post.Property(p => p.Views).HasColumnName("views");
        post.Property(p => p.CreatedAt).HasColumnName("created_at");
    }
}

public class User
{
    public int Id { get; set; }
    public string Username { get; set; } = "";
    public string Email { get; set; } = "";
    public DateTime? LastLogin { get; set; }
    public DateTime CreatedAt { get; set; }
    public string Settings { get; set; } = "";
}

public class Post
{
    public int Id { get; set; }
    public int UserId { get; set; }
    public string Title { get; set; } = "";
    public string Content { get; set; } = "";
    public int Views { get; set; }
    public DateTime CreatedAt { get; set; }
}
