-- Configure SQL Server for benchmarks
IF NOT EXISTS (SELECT 1 FROM sys.databases WHERE name = 'hello_world')
BEGIN
    CREATE DATABASE hello_world;
END
GO

IF EXISTS (SELECT 1 FROM sys.sql_logins WHERE name = 'user')
BEGIN
    DROP LOGIN [user];
END
GO

CREATE LOGIN [user] WITH PASSWORD = 'Benchmark!12345', CHECK_POLICY = OFF, CHECK_EXPIRATION = OFF;
GO

USE hello_world;
GO

IF EXISTS (SELECT 1 FROM sys.database_principals WHERE name = 'user')
BEGIN
    DROP USER [user];
END
GO

CREATE USER [user] FOR LOGIN [user];
ALTER ROLE db_owner ADD MEMBER [user];
GO

-- Schema
IF OBJECT_ID('posts', 'U') IS NOT NULL DROP TABLE posts;
IF OBJECT_ID('users', 'U') IS NOT NULL DROP TABLE users;

CREATE TABLE users (
    id INT IDENTITY(1,1) PRIMARY KEY,
    username NVARCHAR(255) NOT NULL,
    email NVARCHAR(255) NOT NULL UNIQUE,
    created_at DATETIME2 NOT NULL DEFAULT SYSDATETIME(),
    last_login DATETIME2,
    settings NVARCHAR(MAX) NOT NULL
);

CREATE TABLE posts (
    id INT IDENTITY(1,1) PRIMARY KEY,
    user_id INT NOT NULL,
    title NVARCHAR(255) NOT NULL,
    content NVARCHAR(MAX) NOT NULL,
    views INT NOT NULL DEFAULT 0,
    created_at DATETIME2 NOT NULL DEFAULT SYSDATETIME(),
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE INDEX idx_posts_views ON posts(views DESC);
CREATE INDEX idx_posts_user_created ON posts(user_id, created_at DESC);

-- Seeding
SET NOCOUNT ON;

-- Generate 10000 users using a set-based approach for performance
WITH Numbers AS (
    SELECT TOP 10000 ROW_NUMBER() OVER (ORDER BY (SELECT NULL)) AS n
    FROM sys.all_columns c1 CROSS JOIN sys.all_columns c2
)
INSERT INTO users (username, email, created_at, settings)
SELECT 
    CONCAT('user_', n), 
    CONCAT('user_', n, '@example.com'), 
    SYSDATETIME(), 
    '{"theme": "dark", "notifications": true, "language": "en"}'
FROM Numbers
ORDER BY n;

-- Generate posts (15 per user)
WITH PostNumbers AS (
    SELECT TOP 15 ROW_NUMBER() OVER (ORDER BY (SELECT NULL)) AS n
    FROM sys.all_columns
)
INSERT INTO posts (user_id, title, content, views, created_at)
SELECT 
    u.id,
    CONCAT('Post ', pn.n),
    'Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.',
    ABS(CHECKSUM(NEWID())) % 10000,
    DATEADD(minute, -pn.n, SYSDATETIME())
FROM users u
CROSS JOIN PostNumbers pn;

SET NOCOUNT OFF;
GO
