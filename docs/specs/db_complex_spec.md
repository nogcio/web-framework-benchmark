# Database Complex Test Case (Interactive User Profile)

This test case verifies the framework's ability to perform a realistic "Master-Detail" database operation, mixing reads and writes. It maps multiple database rows (including a JSON column and a one-to-many relationship) to a nested JSON response and updates the user's login timestamp.

## Requirements

### Endpoint
- **URL**: `/db/user-profile/:email`
- **Method**: `GET`
- **URL Parameters**:
  - `email`: String (The user's email address, e.g., "user_1@example.com")

### Database Schema
The benchmark relies on two tables: `users` and `posts`.

#### Table: `users`
- `id`: Integer (Primary Key).
- `username`: String (e.g., "user_1")
- `email`: String (Unique Index).
- `created_at`: Timestamp
- `last_login`: Timestamp (Nullable)
- `settings`: JSON / JSONB (contains user preferences)

#### Table: `posts`
- `id`: Integer (Primary Key).
- `user_id`: Integer (Foreign Key to `users.id`)
- `title`: String
- `content`: Text
- `created_at`: Timestamp

#### Data Seeding
- **Users**: 10,000 rows.
  - `id`: 1 to 10,000.
  - `email`: `user_{id}@example.com` (Must be unique).
  - `settings`: `{"theme": "dark", "notifications": true, "language": "en"}`
- **Posts**: Each user must have **15 posts**.
  - `title`: "Post A", "Post B", etc.
  - `content`: A string of ~100 characters.
  - `created_at`: Varied timestamps.

### Processing Logic
1. Parse the `email` from the URL path.
2. **Parallel Execution Recommended**:
   - Query A: Fetch the user by `email`.
   - Query B: Fetch the **5 most popular posts** globally (e.g., `ORDER BY views DESC LIMIT 5`).
3. After Query A completes and we have the user's `id`:
   - Query D: **Update** the user's `last_login` field to the current timestamp (`NOW()`).
   - Query C: Fetch the **10 most recent posts** for this user.
4. Map the results to a nested JSON object.
5. If the user is not found, return `404 Not Found`.

### Response
- **Status Code**: `200 OK`
- **Headers**: `Content-Type: application/json`
- **Body**: JSON object with the following structure:

```json
{
  "username": "user_1",
  "email": "user_1@example.com",
  "createdAt": "2024-01-01T00:00:00Z",
  "lastLogin": "2024-01-01T12:00:00Z",
  "settings": {
    "theme": "dark",
    "notifications": true,
    "language": "en"
  },
  "posts": [
    {
      "id": 150,
      "title": "Post 15",
      "content": "Lorem ipsum...",
      "views": 123,
      "createdAt": "2024-01-15T10:00:00Z"
    },
    // ... 9 more posts
  ],
  "trending": [
    {
      "id": 500,
      "title": "Popular Post",
      "content": "Lorem ipsum...",
      "views": 10000,
      "createdAt": "2024-01-10T10:00:00Z"
    },
    // ... 4 more trending posts
  ]
}
```

## Verification Logic
The test runner performs the following checks:
1. Sends a `GET` request to `/db/user-profile/user_1@example.com`.
2. Asserts `200 OK` and `application/json`.
3. Parses the response.
4. Asserts:
   - `email` is "user_1@example.com".
   - `lastLogin` is present (indicating update).
   - `settings` is an object.
   - `posts` is an array of length 10.
   - `trending` is an array of length 5.

## Reference Implementation Details

### PostgreSQL Schema
```sql
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
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

-- Seeding (Example)
INSERT INTO users (username, email, created_at, settings)
SELECT 
    'user_' || s,
    'user_' || s || '@example.com', 
    NOW(), 
    '{"theme": "dark"}'::jsonb
FROM generate_series(1, 10000) AS s;

INSERT INTO posts (user_id, title, content, views, created_at)
SELECT 
    u.id, 
    'Post ' || p, 
    'Content for post ' || p, 
    (random() * 10000)::int,
    NOW() - (p || ' minutes')::interval
FROM users u
CROSS JOIN generate_series(1, 15) AS p;

-- Index for trending posts
CREATE INDEX idx_posts_views ON posts(views DESC);
```

### MongoDB Document
```json
// users collection
{
  "_id": ObjectId("..."), // Internal ID
  "username": "user_1",
  "email": "user_1@example.com", // Indexed, Unique
  "createdAt": ISODate("..."),
  "settings": { ... }
}

// posts collection
{
  "_id": ObjectId("..."),
  "user_id": ObjectId("..."), // Reference to users._id
  "title": "Post 1",
  "content": "...",
  "views": 123, // Indexed
  "createdAt": ISODate("...")
}
```



### Implementation Notes
- **N+1 Problem**: Implementations should avoid executing 1 query for the user and then 1 query for posts (or worse, 1 query per post).
- **Preferred Approach**:
  - **Single Query (JOIN)**: `SELECT u.*, p.* FROM users u JOIN posts p ...` (Requires careful mapping/deduplication in application code).
  - **Two Queries**: 1. `SELECT * FROM users WHERE id=?` 2. `SELECT * FROM posts WHERE user_id=? ...` (Acceptable, often cleaner).
