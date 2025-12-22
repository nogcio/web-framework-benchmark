# Benchmark Scripts

This directory contains Lua scripts used by `wrk` to generate load and measure performance for different test scenarios.

## Scripts

- **wrk_hello.lua**: Tests the "Hello World" plaintext endpoint.
- **wrk_json.lua**: Tests the JSON serialization endpoint.
- **wrk_db_read_one.lua**: Tests reading a single row from the database.
- **wrk_db_read_paging.lua**: Tests reading multiple rows (paging) from the database.
- **wrk_db_write.lua**: Tests writing/updating rows in the database.
- **wrk_static_files_small.lua**: Tests serving a small static file.
- **wrk_static_files_medium.lua**: Tests serving a medium static file.
- **wrk_static_files_large.lua**: Tests serving a large static file.

## Usage

These scripts are automatically used by the Rust CLI runner when executing benchmarks. You generally do not need to run them manually unless you are debugging or developing new test scenarios.

## Helper Scripts

- **ci_parse_tests.py**: A Python script used in CI/CD pipelines to parse test results.
