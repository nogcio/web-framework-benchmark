FROM mcr.microsoft.com/dotnet/sdk:10.0 AS build
WORKDIR /app

COPY wfb-aspnetcore-grpc.csproj .
RUN dotnet restore

COPY . .
RUN dotnet publish -c Release -o out

FROM mcr.microsoft.com/dotnet/aspnet:10.0
WORKDIR /app
COPY --from=build /app/out .

COPY --from=ghcr.io/grpc-ecosystem/grpc-health-probe:v0.4.24 /ko-app/grpc-health-probe /bin/grpc_health_probe

ENV ASPNETCORE_URLS=http://+:8080

HEALTHCHECK --interval=5s --timeout=3s --retries=3 CMD ["/bin/grpc_health_probe", "-addr=127.0.0.1:8080"]

CMD ["dotnet", "wfb-aspnetcore-grpc.dll"]
