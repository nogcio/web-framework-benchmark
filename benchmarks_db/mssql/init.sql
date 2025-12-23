-- Configure SQL Server for benchmarks
IF NOT EXISTS (SELECT 1 FROM sys.databases WHERE name = 'benchmark')
BEGIN
    CREATE DATABASE benchmark;
END
GO

IF EXISTS (SELECT 1 FROM sys.sql_logins WHERE name = 'benchmark')
BEGIN
    DROP LOGIN benchmark;
END
GO

CREATE LOGIN benchmark WITH PASSWORD = 'Benchmark!12345', CHECK_POLICY = OFF, CHECK_EXPIRATION = OFF;
GO

USE benchmark;
GO

IF EXISTS (SELECT 1 FROM sys.database_principals WHERE name = 'benchmark')
BEGIN
    DROP USER benchmark;
END
GO

CREATE USER benchmark FOR LOGIN benchmark;
ALTER ROLE db_owner ADD MEMBER benchmark;
GO

IF OBJECT_ID('dbo.hello_world', 'U') IS NULL
BEGIN
    CREATE TABLE dbo.hello_world (
        id INT IDENTITY(1,1) PRIMARY KEY,
        name NVARCHAR(255) NOT NULL,
        created_at DATETIME2 NOT NULL,
        updated_at DATETIME2 NOT NULL
    );
END
GO

;WITH numbers AS (
    SELECT TOP (1000) ROW_NUMBER() OVER (ORDER BY (SELECT NULL)) AS n
    FROM sys.objects AS o1
    CROSS JOIN sys.objects AS o2
)
INSERT INTO dbo.hello_world (name, created_at, updated_at)
SELECT
    CONCAT('name_', n) AS name,
    DATEADD(SECOND, -n, SYSUTCDATETIME()) AS created_at,
    DATEADD(SECOND, -(n - 1), SYSUTCDATETIME()) AS updated_at
FROM numbers
WHERE NOT EXISTS (
    SELECT 1 FROM dbo.hello_world hw WHERE hw.id = numbers.n
)
ORDER BY n;
GO

-- Tweet Service Tables
IF OBJECT_ID('dbo.users', 'U') IS NULL
BEGIN
    CREATE TABLE dbo.users (
        id INT IDENTITY(1,1) PRIMARY KEY,
        username NVARCHAR(128) NOT NULL UNIQUE,
        password_hash NVARCHAR(64) NOT NULL
    );
END

IF OBJECT_ID('dbo.tweets', 'U') IS NULL
BEGIN
    CREATE TABLE dbo.tweets (
        id INT IDENTITY(1,1) PRIMARY KEY,
        user_id INT NOT NULL FOREIGN KEY REFERENCES dbo.users(id),
        content NVARCHAR(256) NOT NULL,
        created_at DATETIME2 NOT NULL DEFAULT SYSUTCDATETIME()        
    );
    CREATE INDEX idx_tweets_created_at ON dbo.tweets(created_at DESC);
END

IF OBJECT_ID('dbo.likes', 'U') IS NULL
BEGIN
    CREATE TABLE dbo.likes (
        user_id INT NOT NULL FOREIGN KEY REFERENCES dbo.users(id),
        tweet_id INT NOT NULL FOREIGN KEY REFERENCES dbo.tweets(id),
        PRIMARY KEY (user_id, tweet_id)
    );
    CREATE INDEX idx_likes_tweet_id ON dbo.likes(tweet_id);
END

-- Pre-seed Users
;WITH numbers AS (
    SELECT TOP (1000) ROW_NUMBER() OVER (ORDER BY (SELECT NULL)) AS n
    FROM sys.objects AS o1 CROSS JOIN sys.objects AS o2
)
INSERT INTO dbo.users (username, password_hash)
SELECT CONCAT('user_', n), CONCAT('hash_', n)
FROM numbers
WHERE NOT EXISTS (SELECT 1 FROM dbo.users WHERE username = CONCAT('user_', n));

-- Pre-seed Tweets
;WITH numbers AS (
    SELECT TOP (10000) ROW_NUMBER() OVER (ORDER BY (SELECT NULL)) AS n
    FROM sys.objects AS o1 CROSS JOIN sys.objects AS o2 CROSS JOIN sys.objects AS o3
)
INSERT INTO dbo.tweets (user_id, content, created_at)
SELECT
    ABS(CHECKSUM(NEWID()) % 1000) + 1,
    CONCAT('Tweet content ', n),
    DATEADD(SECOND, -n, SYSUTCDATETIME())
FROM numbers;
