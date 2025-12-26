using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;
using System.IdentityModel.Tokens.Jwt;
using System.Security.Claims;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.AspNetCore.Authentication.JwtBearer;
using Microsoft.AspNetCore.Authorization;
using Microsoft.Data.SqlClient;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;
using Microsoft.IdentityModel.Tokens;
using MySqlConnector;
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

// --- Database Configuration ---
var port = Environment.GetEnvironmentVariable("PORT") ?? "8000";
var dbHost = Environment.GetEnvironmentVariable("DB_HOST") ?? "db";
var dbPortEnv = Environment.GetEnvironmentVariable("DB_PORT") ?? "5432";
var dbUser = Environment.GetEnvironmentVariable("DB_USER") ?? "benchmark";
var dbPass = Environment.GetEnvironmentVariable("DB_PASSWORD") ?? "benchmark";
var dbName = Environment.GetEnvironmentVariable("DB_NAME") ?? "benchmark";
var dbKindEnv = Environment.GetEnvironmentVariable("DB_KIND") ?? "postgres";
var dbKind = dbKindEnv.ToLowerInvariant();
var isMysql = dbKind == "mysql" || dbKind == "mariadb";
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
        SslMode = MySqlSslMode.Preferred,
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
        MinPoolSize = 256,
        MaxPoolSize = 256,
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
        MaxPoolSize = 256,
        MinPoolSize = 256,
        AutoPrepareMinUsages = 2,
        MaxAutoPrepare = 128,
    };
    builder.Services.AddDbContextPool<BenchmarkContext>(options =>
        options.UseNpgsql(csb.ConnectionString, npgsql => npgsql.EnableRetryOnFailure()));
}

// --- Auth Configuration ---
var jwtKey = Encoding.UTF8.GetBytes("super_secret_key_for_benchmarking_only_12345");
builder.Services.AddAuthentication(JwtBearerDefaults.AuthenticationScheme)
    .AddJwtBearer(options =>
    {
        options.TokenValidationParameters = new TokenValidationParameters
        {
            ValidateIssuer = false,
            ValidateAudience = false,
            ValidateLifetime = true,
            ValidateIssuerSigningKey = true,
            IssuerSigningKey = new SymmetricSecurityKey(jwtKey)
        };
    });
builder.Services.AddAuthorization();

var app = builder.Build();

app.Use(async (context, next) =>
{
    if (context.Request.Headers.TryGetValue("x-request-id", out var requestId))
    {
        context.Response.Headers.Append("x-request-id", requestId);
    }
    await next();
});

app.UseAuthentication();
app.UseAuthorization();

// --- Standard Benchmark Endpoints ---

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
        .Select(r => new { id = r.Id, name = r.Name, createdAt = r.CreatedAt, updatedAt = r.UpdatedAt })
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
        .Select(r => new { id = r.Id, name = r.Name, createdAt = r.CreatedAt, updatedAt = r.UpdatedAt })
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

    var result = new { id = entity.Id, name = entity.Name, createdAt = entity.CreatedAt, updatedAt = entity.UpdatedAt };
    return Results.Json(result);
});

// --- Tweet Service Endpoints ---

// Register
app.MapPost("/api/auth/register", async (RegisterRequest req, BenchmarkContext db) =>
{
    using var sha256 = SHA256.Create();
    var hashBytes = sha256.ComputeHash(Encoding.UTF8.GetBytes(req.Password));
    var hash = Convert.ToBase64String(hashBytes);

    var user = new User { Username = req.Username, PasswordHash = hash };
    try
    {
        db.Users.Add(user);
        await db.SaveChangesAsync();
        return Results.Created($"/api/users/{user.Id}", null);
    }
    catch
    {
        return Results.Conflict();
    }
});

// Login
app.MapPost("/api/auth/login", async (LoginRequest req, BenchmarkContext db) =>
{
    var user = await db.Users.FirstOrDefaultAsync(u => u.Username == req.Username);
    if (user == null) return Results.Unauthorized();

    using var sha256 = SHA256.Create();
    var hashBytes = sha256.ComputeHash(Encoding.UTF8.GetBytes(req.Password));
    var hash = Convert.ToBase64String(hashBytes);

    if (user.PasswordHash != hash) return Results.Unauthorized();

    var tokenHandler = new JwtSecurityTokenHandler();
    var tokenDescriptor = new SecurityTokenDescriptor
    {
        Subject = new ClaimsIdentity(new[]
        {
            new Claim(ClaimTypes.NameIdentifier, user.Id.ToString()),
            new Claim(ClaimTypes.Name, user.Username)
        }),
        Expires = DateTime.UtcNow.AddHours(1),
        SigningCredentials = new SigningCredentials(new SymmetricSecurityKey(jwtKey), SecurityAlgorithms.HmacSha256Signature)
    };
    var token = tokenHandler.CreateToken(tokenDescriptor);
    return Results.Ok(new { token = tokenHandler.WriteToken(token) });
});

// Feed
app.MapGet("/api/feed", [Authorize] async (BenchmarkContext db) =>
{
    var tweets = await db.Tweets
        .AsNoTracking()
        .OrderByDescending(t => t.CreatedAt)
        .Take(20)
        .Select(t => new TweetDto
        {
            Id = t.Id,
            Username = t.User.Username,
            Content = t.Content,
            CreatedAt = t.CreatedAt,
            Likes = t.Likes.Count
        })
        .ToListAsync();
    return Results.Ok(tweets);
});

// Get Tweet
app.MapGet("/api/tweets/{id:int}", [Authorize] async (int id, BenchmarkContext db) =>
{
    var tweet = await db.Tweets
        .AsNoTracking()
        .Where(t => t.Id == id)
        .Select(t => new TweetDto
        {
            Id = t.Id,
            Username = t.User.Username,
            Content = t.Content,
            CreatedAt = t.CreatedAt,
            Likes = t.Likes.Count
        })
        .FirstOrDefaultAsync();
    return tweet is null ? Results.NotFound() : Results.Ok(tweet);
});

// Create Tweet
app.MapPost("/api/tweets", [Authorize] async (CreateTweetRequest req, ClaimsPrincipal user, BenchmarkContext db) =>
{
    var userId = int.Parse(user.FindFirst(ClaimTypes.NameIdentifier)!.Value);
    var tweet = new Tweet
    {
        UserId = userId,
        Content = req.Content,
        CreatedAt = DateTime.UtcNow
    };
    db.Tweets.Add(tweet);
    await db.SaveChangesAsync();
    return Results.Created($"/api/tweets/{tweet.Id}", new { id = tweet.Id });
});

// Like Tweet (Toggle)
app.MapPost("/api/tweets/{id:int}/like", [Authorize] async (int id, ClaimsPrincipal user, BenchmarkContext db) =>
{
    var userId = int.Parse(user.FindFirst(ClaimTypes.NameIdentifier)!.Value);

    // Check if tweet exists
    if (!await db.Tweets.AnyAsync(t => t.Id == id))
    {
        return Results.NotFound();
    }
    
    // Check if like exists
    var existingLike = await db.Likes.FindAsync(userId, id);
    if (existingLike != null)
    {
        db.Likes.Remove(existingLike);
    }
    else
    {
        db.Likes.Add(new Like { UserId = userId, TweetId = id });
    }

    try
    {
        await db.SaveChangesAsync();
        return Results.Ok();
    }
    catch (DbUpdateConcurrencyException)
    {
        return Results.Conflict();
    }
    catch (DbUpdateException)
    {
        return Results.Conflict();
    }
});

app.Run($"http://0.0.0.0:{port}");

// --- Entities & Context ---

public class BenchmarkContext : DbContext
{
    public BenchmarkContext(DbContextOptions<BenchmarkContext> options) : base(options) { }

    public DbSet<HelloWorld> HelloWorld => Set<HelloWorld>();
    public DbSet<User> Users => Set<User>();
    public DbSet<Tweet> Tweets => Set<Tweet>();
    public DbSet<Like> Likes => Set<Like>();

    protected override void OnModelCreating(ModelBuilder modelBuilder)
    {
        // Standard Benchmark Table
        var helloWorld = modelBuilder.Entity<HelloWorld>();
        helloWorld.ToTable("hello_world");
        helloWorld.HasKey(e => e.Id);
        helloWorld.Property(e => e.Id).HasColumnName("id");
        helloWorld.Property(e => e.Name).HasColumnName("name");
        helloWorld.Property(e => e.CreatedAt).HasColumnName("created_at");
        helloWorld.Property(e => e.UpdatedAt).HasColumnName("updated_at");
        // Tweet Service Tables
        modelBuilder.Entity<User>().ToTable("users");
        modelBuilder.Entity<User>().Property(u => u.Id).HasColumnName("id");
        modelBuilder.Entity<User>().Property(u => u.Username).HasColumnName("username");
        modelBuilder.Entity<User>().Property(u => u.PasswordHash).HasColumnName("password_hash");

        modelBuilder.Entity<Tweet>().ToTable("tweets");
        modelBuilder.Entity<Tweet>().Property(t => t.Id).HasColumnName("id");
        modelBuilder.Entity<Tweet>().Property(t => t.UserId).HasColumnName("user_id");
        modelBuilder.Entity<Tweet>().Property(t => t.Content).HasColumnName("content");
        modelBuilder.Entity<Tweet>().Property(t => t.CreatedAt).HasColumnName("created_at");

        modelBuilder.Entity<Like>().ToTable("likes");
        modelBuilder.Entity<Like>().HasKey(l => new { l.UserId, l.TweetId });
        modelBuilder.Entity<Like>().Property(l => l.UserId).HasColumnName("user_id");
        modelBuilder.Entity<Like>().Property(l => l.TweetId).HasColumnName("tweet_id");
    }
}

public class HelloWorld
{
    public int Id { get; set; }
    public string Name { get; set; } = string.Empty;
    public DateTime CreatedAt { get; set; }
    public DateTime UpdatedAt { get; set; }
}

public class User
{
    public int Id { get; set; }
    public string Username { get; set; } = "";
    public string PasswordHash { get; set; } = "";
}

public class Tweet
{
    public int Id { get; set; }
    public int UserId { get; set; }
    public User User { get; set; } = null!;
    public string Content { get; set; } = "";
    public DateTime CreatedAt { get; set; }
    public ICollection<Like> Likes { get; set; } = new List<Like>();
}

public class Like
{
    public int UserId { get; set; }
    public int TweetId { get; set; }
}

public record RegisterRequest(string Username, string Password);
public record LoginRequest(string Username, string Password);
public record CreateTweetRequest(string Content);
public class TweetDto
{
    public int Id { get; set; }
    public string Username { get; set; } = "";
    public string Content { get; set; } = "";
    public DateTime CreatedAt { get; set; }
    public int Likes { get; set; }
}
