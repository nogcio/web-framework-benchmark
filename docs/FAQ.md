# FAQ

## Is this a “framework ranking”?

No. WFB is an open benchmark suite + runner. The results are a snapshot of **specific workloads** on **specific environments**.

## Why not only do “Hello World”?

Because it mostly measures minimal HTTP overhead. WFB focuses on workloads that include JSON parsing/serialization, aggregation logic, database access patterns, and static file semantics.

## How do you ensure correctness?

[nogcio/wrkr](https://github.com/nogcio/wrkr) (via our Lua scenarios under `scripts/`) validates every response against the test requirements during load (not only before the benchmark). Incorrect aggregates, missing fields, wrong headers, unstable file bytes, etc. are treated as failures.

## What is the warmup and why is it needed?

Every test includes a 30s warmup phase to reduce cold-start / JIT bias.

## What do “connections” mean here?

Connections are virtual users (VUs). Tests are executed with a `ramping-vus` profile up to a configured max VU target.

## What does latency measure and in what units?

Latency is measured in the load generator as wall-clock elapsed time per request and stored in microseconds (the runner converts from the load generator’s milliseconds). Percentiles are reported from an HDRHistogram.

## Is keep-alive used?

Yes (the client is connection-oriented). For gRPC, HTTP/2 is used.

## Why not tune each framework to its absolute peak?

Per-framework hand-tuning often becomes a contest of configuration expertise rather than a comparison of defaults and typical production setups. WFB prefers a consistent baseline with a clear spec.

If you want a tuned profile, propose it as an additional benchmark variant and make it reproducible.

## Can a framework cache results?

No for the “logic” tests. The scenarios vary inputs and validate outputs so caching shortcuts either won’t help or will fail validation.

## How do I reproduce results?

Start with the Quick Start in the README and run:

- `cargo run --release --bin wfb-runner -- run <run_id> --env local`

For correctness verification (no load test, just spec checks):

- `cargo run --release --bin wfb-runner -- verify --benchmark <benchmark_name> --env local`

For single-benchmark development / debugging:

- `cargo run --release --bin wfb-runner -- dev <benchmark_name> --env local`

`dev` starts the app (and DB if needed) and tails logs; it does **not** run the benchmark load.

If you want to run load against a single benchmark locally, start it with `dev` and then run [nogcio/wrkr](https://github.com/nogcio/wrkr) manually against the printed URL.

Note: `static_files` is now enabled in the runner.

## I think a benchmark is unfair / wrong. What should I do?

Open an issue with:

- what you believe is incorrect,
- a minimal reproduction,
- a proposed change (spec, script, config, or implementation).

We prefer concrete fixes over debates.
