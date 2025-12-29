using System.Text.Json;
using System.Text.Json.Serialization;
using System.Security.Claims;
using System.Security.Cryptography;
using System.Text;
using System.IdentityModel.Tokens.Jwt;
using Microsoft.AspNetCore.Authentication.JwtBearer;
using Microsoft.AspNetCore.Authorization;
using Microsoft.Extensions.Logging;
using Microsoft.IdentityModel.Tokens;
using MongoDB.Bson;
using MongoDB.Bson.Serialization.Attributes;
using MongoDB.Driver;

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
var dbPort = Environment.GetEnvironmentVariable("DB_PORT") ?? "27017";
var dbUser = Environment.GetEnvironmentVariable("DB_USER") ?? "benchmark";
var dbPass = Environment.GetEnvironmentVariable("DB_PASSWORD") ?? "benchmark";
var dbName = Environment.GetEnvironmentVariable("DB_NAME") ?? "benchmark";

var connectionString = $"mongodb://{dbUser}:{dbPass}@{dbHost}:{dbPort}/{dbName}?authSource=admin&maxPoolSize=256&minPoolSize=256&waitQueueTimeoutMS=150";

var client = new MongoClient(connectionString);
var database = client.GetDatabase(dbName);

builder.Services.AddSingleton(database);
builder.Services.AddSingleton(database.GetCollection<HelloWorld>("hello_world"));
builder.Services.AddSingleton(database.GetCollection<User>("users"));
builder.Services.AddSingleton(database.GetCollection<Tweet>("tweets"));
builder.Services.AddSingleton(database.GetCollection<Like>("likes"));

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

app.UseAuthentication();
app.UseAuthorization();

app.MapGet("/health", async (IMongoDatabase db) =>
{
    try
    {
        await db.RunCommandAsync((Command<BsonDocument>)"{ping:1}");
        return Results.Text("OK");
    }
    catch
    {
        return Results.Text("Service Unavailable", statusCode: 503);
    }
});

// --- Benchmarks ---

app.MapGet("/db/read/one", async (string id, IMongoCollection<HelloWorld> col) =>
{
    if (!ObjectId.TryParse(id, out var objectId))
    {
        return Results.BadRequest("Invalid ObjectId");
    }

    var filter = Builders<HelloWorld>.Filter.Eq(x => x.Id, objectId);
    var result = await col.Find(filter).FirstOrDefaultAsync();

    if (result != null)
    {
        return Results.Json(result);
    }
    return Results.NotFound();
});

app.MapGet("/db/read/many", async (int offset, int? limit, IMongoCollection<HelloWorld> col) =>
{
    var actualLimit = limit ?? 50;
    var results = await col.Find(Builders<HelloWorld>.Filter.Empty)
                           .Skip(offset)
                           .Limit(actualLimit)
                           .ToListAsync();
    return Results.Json(results);
});

app.MapPost("/db/write/insert", async (InsertRequest req, IMongoCollection<HelloWorld> col) =>
{
    var doc = new HelloWorld
    {
        Id = ObjectId.GenerateNewId(),
        Name = req.Name,
        CreatedAt = DateTime.UtcNow,
        UpdatedAt = DateTime.UtcNow
    };

    await col.InsertOneAsync(doc);
    return Results.Json(doc);
});

// --- Tweet Service ---

// Register
app.MapPost("/api/auth/register", async (RegisterRequest req, IMongoCollection<User> users) =>
{
    using var sha256 = SHA256.Create();
    var hashBytes = sha256.ComputeHash(Encoding.UTF8.GetBytes(req.Password));
    var hash = Convert.ToBase64String(hashBytes);

    var user = new User { Id = ObjectId.GenerateNewId(), Username = req.Username, PasswordHash = hash };
    try
    {
        await users.InsertOneAsync(user);
        return Results.Created($"/api/users/{user.Id}", null);
    }
    catch (MongoWriteException ex) when (ex.WriteError.Category == ServerErrorCategory.DuplicateKey)
    {
        return Results.Conflict();
    }
});

// Login
app.MapPost("/api/auth/login", async (LoginRequest req, IMongoCollection<User> users) =>
{
    var user = await users.Find(u => u.Username == req.Username).FirstOrDefaultAsync();
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
app.MapGet("/api/feed", [Authorize] async (IMongoCollection<Tweet> tweets) =>
{
    // The C# driver LINQ or fluent API for aggregation can be verbose. 
    // Let's use BsonDocument for the pipeline to be precise and efficient.
    
    var aggregation = tweets.Aggregate()
        .SortByDescending(t => t.CreatedAt)
        .Limit(20)
        .Lookup("users", "user_id", "_id", "user")
        .Unwind("user")
        .Lookup("likes", "_id", "tweet_id", "likes")
        .Project(new BsonDocument
        {
            { "id", "$_id" },
            { "username", "$user.username" },
            { "content", "$content" },
            { "created_at", "$created_at" },
            { "likes", new BsonDocument("$size", "$likes") }
        });

    var results = await aggregation.ToListAsync();
    
    // Convert BsonDocument to DTO or just return JSON from BsonDocument?
    // Results.Json can handle object, but BsonDocument might need conversion.
    // Let's map to Dto manually.
    var dtos = results.Select(doc => new TweetDto
    {
        Id = doc["id"].AsObjectId.ToString(),
        Username = doc["username"].AsString,
        Content = doc["content"].AsString,
        CreatedAt = doc["created_at"].ToUniversalTime(),
        Likes = doc["likes"].AsInt32
    });

    return Results.Ok(dtos);
});

// Get Tweet
app.MapGet("/api/tweets/{id}", [Authorize] async (string id, IMongoCollection<Tweet> tweets) =>
{
    if (!ObjectId.TryParse(id, out var objectId)) return Results.BadRequest();

    var aggregation = tweets.Aggregate()
        .Match(t => t.Id == objectId)
        .Lookup("users", "user_id", "_id", "user")
        .Unwind("user")
        .Lookup("likes", "_id", "tweet_id", "likes")
        .Project(new BsonDocument
        {
            { "id", "$_id" },
            { "username", "$user.username" },
            { "content", "$content" },
            { "created_at", "$created_at" },
            { "likes", new BsonDocument("$size", "$likes") }
        });

    var doc = await aggregation.FirstOrDefaultAsync();
    if (doc == null) return Results.NotFound();

    var dto = new TweetDto
    {
        Id = doc["id"].AsObjectId.ToString(),
        Username = doc["username"].AsString,
        Content = doc["content"].AsString,
        CreatedAt = doc["created_at"].ToUniversalTime(),
        Likes = doc["likes"].AsInt32
    };

    return Results.Ok(dto);
});

// Create Tweet
app.MapPost("/api/tweets", [Authorize] async (CreateTweetRequest req, ClaimsPrincipal user, IMongoCollection<Tweet> tweets) =>
{
    var userIdStr = user.FindFirst(ClaimTypes.NameIdentifier)!.Value;
    var userId = ObjectId.Parse(userIdStr);
    
    var tweet = new Tweet
    {
        Id = ObjectId.GenerateNewId(),
        UserId = userId,
        Content = req.Content,
        CreatedAt = DateTime.UtcNow
    };
    
    await tweets.InsertOneAsync(tweet);
    return Results.Created($"/api/tweets/{tweet.Id}", new { id = tweet.Id.ToString() });
});

// Like Tweet (Toggle)
app.MapPost("/api/tweets/{id}/like", [Authorize] async (string id, ClaimsPrincipal user, IMongoCollection<Tweet> tweets, IMongoCollection<Like> likes) =>
{
    if (!ObjectId.TryParse(id, out var tweetId)) return Results.BadRequest();
    var userIdStr = user.FindFirst(ClaimTypes.NameIdentifier)!.Value;
    var userId = ObjectId.Parse(userIdStr);

    // Check if tweet exists
    var tweetExists = await tweets.Find(t => t.Id == tweetId).AnyAsync();
    if (!tweetExists) return Results.NotFound();

    // Check if like exists
    var existingLike = await likes.Find(l => l.UserId == userId && l.TweetId == tweetId).FirstOrDefaultAsync();
    
    if (existingLike != null)
    {
        await likes.DeleteOneAsync(l => l.UserId == userId && l.TweetId == tweetId);
    }
    else
    {
        try 
        {
            await likes.InsertOneAsync(new Like { UserId = userId, TweetId = tweetId });
        }
        catch (MongoWriteException ex) when (ex.WriteError.Category == ServerErrorCategory.DuplicateKey)
        {
            // Race condition, already liked
        }
    }

    return Results.Ok();
});

app.Run($"http://0.0.0.0:{port}");

// --- Models ---

public class HelloWorld
{
    [BsonId]
    [JsonConverter(typeof(ObjectIdConverter))]
    public ObjectId Id { get; set; }

    [BsonElement("name")]
    public string Name { get; set; } = "";

    [BsonElement("created_at")]
    public DateTime CreatedAt { get; set; }

    [BsonElement("updated_at")]
    public DateTime UpdatedAt { get; set; }
}

public class User
{
    [BsonId]
    public ObjectId Id { get; set; }
    [BsonElement("username")]
    public string Username { get; set; } = "";
    [BsonElement("password_hash")]
    public string PasswordHash { get; set; } = "";
}

public class Tweet
{
    [BsonId]
    public ObjectId Id { get; set; }
    [BsonElement("user_id")]
    public ObjectId UserId { get; set; }
    [BsonElement("content")]
    public string Content { get; set; } = "";
    [BsonElement("created_at")]
    public DateTime CreatedAt { get; set; }
}

public class Like
{
    [BsonId]
    public ObjectId Id { get; set; } // MongoDB needs an ID for the document, even if we don't use it
    [BsonElement("user_id")]
    public ObjectId UserId { get; set; }
    [BsonElement("tweet_id")]
    public ObjectId TweetId { get; set; }
}

public class InsertRequest
{
    public string Name { get; set; } = "";
}

public class RegisterRequest
{
    public string Username { get; set; } = "";
    public string Password { get; set; } = "";
}

public class LoginRequest
{
    public string Username { get; set; } = "";
    public string Password { get; set; } = "";
}

public class CreateTweetRequest
{
    public string Content { get; set; } = "";
}

public class TweetDto
{
    public string Id { get; set; } = "";
    public string Username { get; set; } = "";
    public string Content { get; set; } = "";
    public DateTime CreatedAt { get; set; }
    public int Likes { get; set; }
}

public class ObjectIdConverter : JsonConverter<ObjectId>
{
    public override ObjectId Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        return ObjectId.Parse(reader.GetString());
    }

    public override void Write(Utf8JsonWriter writer, ObjectId value, JsonSerializerOptions options)
    {
        writer.WriteStringValue(value.ToString());
    }
}
