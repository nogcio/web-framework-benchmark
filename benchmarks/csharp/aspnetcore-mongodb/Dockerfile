FROM mcr.microsoft.com/dotnet/sdk:10.0 AS build
WORKDIR /app

COPY wfb-aspnetcore-mongodb.csproj .
RUN dotnet restore

COPY . .
RUN dotnet publish -c Release -o out

FROM mcr.microsoft.com/dotnet/aspnet:10.0
WORKDIR /app
COPY --from=build /app/out .

ENV ASPNETCORE_URLS=http://+:8000
EXPOSE 8000

ENTRYPOINT ["dotnet", "wfb-aspnetcore-mongodb.dll"]
