FROM mcr.microsoft.com/dotnet/sdk:10.0 AS build
WORKDIR /app

COPY wfb-aspnetcore.csproj .
RUN dotnet restore

COPY . .
RUN dotnet publish -c Release -o out

FROM mcr.microsoft.com/dotnet/aspnet:10.0
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/out .
COPY benchmarks_data /app/benchmarks_data

# Install curl for healthcheck if needed, but usually not required inside container for this benchmark
# The benchmark runner uses curl from outside.

ENV ASPNETCORE_URLS=http://+:8080

HEALTHCHECK --interval=5s --timeout=3s --retries=3 CMD curl --fail http://localhost:8080/health || exit 1

CMD ["dotnet", "wfb-aspnetcore.dll"]
