using System.Text.Json;
using System.Text.Json.Serialization;
using System.Security.Claims;
using System.Text;
using Microsoft.Extensions.Logging;
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

var port = Environment.GetEnvironmentVariable("PORT") ?? "8080";
var dbHost = Environment.GetEnvironmentVariable("DB_HOST") ?? "db";
var dbPort = Environment.GetEnvironmentVariable("DB_PORT") ?? "27017";
var dbUser = Environment.GetEnvironmentVariable("DB_USER") ?? "benchmark";
var dbPass = Environment.GetEnvironmentVariable("DB_PASSWORD") ?? "benchmark";
var dbName = Environment.GetEnvironmentVariable("DB_NAME") ?? "hello_world";
var dbPoolSize = Environment.GetEnvironmentVariable("DB_POOL_SIZE") ?? "256";

var connectionString = $"mongodb://{dbUser}:{dbPass}@{dbHost}:{dbPort}/{dbName}?authSource=admin&maxPoolSize={dbPoolSize}&minPoolSize={dbPoolSize}&waitQueueTimeoutMS=5000";

var client = new MongoClient(connectionString);
var database = client.GetDatabase(dbName);

builder.Services.AddSingleton(database);
var usersCollection = database.GetCollection<User>("users");
var postsCollection = database.GetCollection<Post>("posts");
builder.Services.AddSingleton(usersCollection);
builder.Services.AddSingleton(postsCollection);

var app = builder.Build();

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
// ... (omitted)

async Task<IResult> GetUserProfile(string email, IMongoCollection<User> users, IMongoCollection<Post> posts)
{
    // 1. Fetch user
    var user = await users.Find(u => u.Email == email).FirstOrDefaultAsync();
    if (user == null) return Results.NotFound();

    // 2. Update LastLogin
    var update = Builders<User>.Update.Set(u => u.LastLogin, DateTime.UtcNow);
    await users.UpdateOneAsync(u => u.Id == user.Id, update);

    // 3. Fetch trending posts (top 5 by views)
    var trending = await posts.Find(Builders<Post>.Filter.Empty)
        .SortByDescending(p => p.Views)
        .Limit(5)
        .ToListAsync();

    // 4. Fetch user posts (latest 10)
    var userPosts = await posts.Find(p => p.UserId == user.Id)
        .SortByDescending(p => p.CreatedAt)
        .Limit(10)
        .ToListAsync();

    // 5. Return JSON
    return Results.Ok(new
    {
        username = user.Username,
        email = user.Email,
        createdAt = user.CreatedAt.ToString("yyyy-MM-ddTHH:mm:ssZ"),
        lastLogin = DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ssZ"), // Approximate, since we just updated it
        settings = user.Settings, // BsonDocument serializes to JSON
        posts = userPosts.Select(p => new
        {
            id = p.Id.ToString(),
            title = p.Title,
            content = p.Content,
            views = p.Views,
            createdAt = p.CreatedAt.ToString("yyyy-MM-ddTHH:mm:ssZ")
        }),
        trending = trending.Select(p => new
        {
            id = p.Id.ToString(),
            title = p.Title,
            content = p.Content,
            views = p.Views,
            createdAt = p.CreatedAt.ToString("yyyy-MM-ddTHH:mm:ssZ")
        })
    });
}

app.MapGet("/db/user-profile/{email}", async (string email, IMongoCollection<User> users, IMongoCollection<Post> posts) =>
{
    return await GetUserProfile(email, users, posts);
});

app.Run($"http://0.0.0.0:{port}");



public class User
{
    [BsonId]
    public ObjectId Id { get; set; }
    [BsonElement("username")]
    public string Username { get; set; } = "";
    [BsonElement("email")]
    public string Email { get; set; } = "";
    [BsonElement("created_at")]
    public DateTime CreatedAt { get; set; }
    [BsonElement("last_login")]
    public DateTime? LastLogin { get; set; }
    [BsonElement("settings")]
    public object? Settings { get; set; } // Use object to let driver handle BsonDocument/Dictionary mapping
}

public class Post
{
    [BsonId]
    public ObjectId Id { get; set; }
    [BsonElement("user_id")]
    public ObjectId UserId { get; set; }
    [BsonElement("title")]
    public string Title { get; set; } = "";
    [BsonElement("content")]
    public string Content { get; set; } = "";
    [BsonElement("views")]
    public int Views { get; set; }
    [BsonElement("created_at")]
    public DateTime CreatedAt { get; set; }
}
