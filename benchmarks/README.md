# Framework Benchmarks

This directory contains the source code for the various framework implementations being benchmarked.

## Structure

The directory is organized by programming language, and then by framework:

```
benchmarks/
└── <language>/          # e.g., csharp, python, go
    └── <framework>/     # e.g., aspnetcore, fastapi, gin
        ├── Dockerfile   # Instructions to build the benchmark container
        └── src/         # Source code for the benchmark application
```

## Existing Benchmarks

### C#
- **aspnetcore**: Standard ASP.NET Core implementation.
- **aspnetcore-efcore**: ASP.NET Core with Entity Framework Core.
- **aspnetcore-mssql**: ASP.NET Core using MSSQL.
- **aspnetcore-mysql**: ASP.NET Core using MySQL.
- **aspnetcore-npgsql**: ASP.NET Core using PostgreSQL (Npgsql).

## Adding a New Benchmark

To add a new framework, please refer to the [Adding a New Framework](../ADDING_FRAMEWORK.md) guide in the root directory.
