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
