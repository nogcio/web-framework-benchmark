ALTER SYSTEM SET max_connections = 1024;

--
-- Schema for Complex Read Test (User Profile)
--

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    last_login TIMESTAMP,
    settings JSONB NOT NULL
);

CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL REFERENCES users(id),
    title VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    views INT NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Index for trending posts
CREATE INDEX idx_posts_views ON posts(views DESC);
-- Index for user posts (by date)
CREATE INDEX idx_posts_user_created ON posts(user_id, created_at DESC);

--
-- Seeding Data (10,000 users, 150,000 posts)
--

INSERT INTO users (username, email, created_at, settings)
SELECT 
    'user_' || s,
    'user_' || s || '@example.com', 
    NOW(), 
    '{"theme": "dark", "notifications": true, "language": "en"}'::jsonb
FROM generate_series(1, 10000) AS s;

INSERT INTO posts (user_id, title, content, views, created_at)
SELECT 
    u.id, 
    'Post ' || p, 
    'Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.', 
    (random() * 10000)::int,
    NOW() - (p || ' minutes')::interval
FROM users u
CROSS JOIN generate_series(1, 15) AS p;
