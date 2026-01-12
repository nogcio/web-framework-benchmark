DROP TABLE IF EXISTS posts;
DROP TABLE IF EXISTS users;

CREATE TABLE users (
    id INT AUTO_INCREMENT PRIMARY KEY,
    username VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_login DATETIME,
    settings JSON NOT NULL
);

CREATE TABLE posts (
    id INT AUTO_INCREMENT PRIMARY KEY,
    user_id INT NOT NULL,
    title VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    views INT NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE INDEX idx_posts_views ON posts(views DESC);
CREATE INDEX idx_posts_user_created ON posts(user_id, created_at DESC);

-- Optimized data seeding using CTEs (Common Table Expressions) for bulk inserts
-- This is significantly faster than row-by-row insertion in a loop

INSERT INTO users (username, email, created_at, settings)
WITH digits AS (
    SELECT 0 AS d UNION ALL SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3 UNION ALL SELECT 4 UNION ALL SELECT 5 UNION ALL SELECT 6 UNION ALL SELECT 7 UNION ALL SELECT 8 UNION ALL SELECT 9
),
seq AS (
    SELECT d1.d + d2.d * 10 + d3.d * 100 + d4.d * 1000 + 1 AS n
    FROM digits d1
    CROSS JOIN digits d2
    CROSS JOIN digits d3
    CROSS JOIN digits d4
)
SELECT 
    CONCAT('user_', n), 
    CONCAT('user_', n, '@example.com'), 
    NOW(), 
    '{"theme": "dark", "notifications": true, "language": "en"}'
FROM seq;

INSERT INTO posts (user_id, title, content, views, created_at)
WITH RECURSIVE seq_posts AS (
    SELECT 1 AS n
    UNION ALL
    SELECT n + 1 FROM seq_posts WHERE n < 15
)
SELECT 
    u.id, 
    CONCAT('Post ', p.n), 
    'Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.', 
    FLOOR(RAND() * 10000), 
    DATE_SUB(NOW(), INTERVAL p.n MINUTE)
FROM users u
CROSS JOIN seq_posts p;

