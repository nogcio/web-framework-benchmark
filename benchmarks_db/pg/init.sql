ALTER SYSTEM SET max_connections = 1024;

-- Create table and populate with 1000 rows for benchmarks
CREATE TABLE IF NOT EXISTS hello_world (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);

-- Insert 1000 rows with names name_1 .. name_1000 and sample timestamps
INSERT INTO hello_world (name, created_at, updated_at)
SELECT
    'name_' || gs AS name,
    (NOW() - (gs || ' seconds')::interval) AS created_at,
    (NOW() - ((gs - 1) || ' seconds')::interval) AS updated_at
FROM generate_series(1, 1000) AS gs;

-- Tweet Service Tables
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(128) NOT NULL UNIQUE,
    password_hash VARCHAR(64) NOT NULL
);

CREATE TABLE IF NOT EXISTS tweets (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL REFERENCES users(id),
    content VARCHAR(256) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_tweets_created_at ON tweets(created_at DESC);

CREATE TABLE IF NOT EXISTS likes (
    user_id INT NOT NULL REFERENCES users(id),
    tweet_id INT NOT NULL REFERENCES tweets(id),
    PRIMARY KEY (user_id, tweet_id)
);
CREATE INDEX IF NOT EXISTS idx_likes_tweet_id ON likes(tweet_id);

-- Pre-seed Users (1000)
INSERT INTO users (username, password_hash)
SELECT 'user_' || gs, 'hash_' || gs
FROM generate_series(1, 1000) AS gs
ON CONFLICT DO NOTHING;

-- Pre-seed Tweets (10000)
INSERT INTO tweets (user_id, content, created_at)
SELECT
    (random() * 999 + 1)::INT,
    'Tweet content ' || gs,
    NOW() - (gs || ' seconds')::interval
FROM generate_series(1, 10000) AS gs;
