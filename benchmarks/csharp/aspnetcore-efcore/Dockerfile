FROM mcr.microsoft.com/dotnet/sdk:10.0 AS build
WORKDIR /app

COPY wfb-aspnetcore-efcore.csproj .
RUN dotnet restore

COPY . .
RUN dotnet publish -c Release -o out

FROM mcr.microsoft.com/dotnet/aspnet:10.0
WORKDIR /app

# Required for Npgsql kerberos/gssapi dependency to avoid runtime missing libgssapi_krb5.so.2
RUN apt-get update \
	&& apt-get install -y --no-install-recommends libgssapi-krb5-2 \
	&& rm -rf /var/lib/apt/lists/*

COPY --from=build /app/out .

ENV DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=0

ENV ASPNETCORE_URLS=http://+:8000
CMD ["dotnet", "wfb-aspnetcore-efcore.dll"]
