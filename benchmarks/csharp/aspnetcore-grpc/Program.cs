using Wfb.AspNetCore.Grpc.Services;

var builder = WebApplication.CreateBuilder(args);

builder.WebHost.ConfigureKestrel(options =>
{
    options.Limits.MaxConcurrentConnections = null;
    options.Limits.MaxConcurrentUpgradedConnections = null;
    options.Limits.Http2.MaxStreamsPerConnection = 256;
    options.Limits.Http2.InitialStreamWindowSize = 1 * 1024 * 1024; // 1MB
    options.Limits.Http2.InitialConnectionWindowSize = 10 * 1024 * 1024; // 10MB
    options.ListenAnyIP(8080, listenOptions =>
    {
        listenOptions.Protocols = Microsoft.AspNetCore.Server.Kestrel.Core.HttpProtocols.Http2;
    });
});

builder.Logging.ClearProviders();

builder.Services.AddGrpc();
builder.Services.AddGrpcHealthChecks()
    .AddCheck("Health", () => Microsoft.Extensions.Diagnostics.HealthChecks.HealthCheckResult.Healthy());

var app = builder.Build();

app.MapGrpcService<AnalyticsService>();
app.MapGrpcHealthChecksService();
app.MapGet("/", () => "Communication with gRPC endpoints must be made through a gRPC client.");

app.Run();
