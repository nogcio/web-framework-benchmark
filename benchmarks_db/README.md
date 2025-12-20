# benchmark_db

Postgres Docker image used for the `DbReadOne`, `DbReadPaging`, `DbWrite` benchmark.

Build and run:

```
docker build -t benchmark_db:latest .
docker run --name benchmark_db -p 5432:5432 -d benchmark_db:latest
```

Defaults (set in the image):

- database: `benchmark`
- user: `benchmark`
- password: `benchmark`

The `init.sql` script creates the `hello_world` table and inserts 1000 rows (`name_1`..`name_1000`).
