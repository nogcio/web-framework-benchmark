# Build stage
FROM elixir:1.18-alpine AS build

# Install build dependencies
RUN apk add --no-cache build-base git

WORKDIR /app

# Install hex + rebar
RUN mix local.hex --force && \
    mix local.rebar --force

# Set build ENV
ENV MIX_ENV=prod

COPY mix.exs ./
# We might not have mix.lock initially, but assuming we generate it or verify it works.
# If I create mix.exs manually, I won't have mix.lock. `mix deps.get` will generate it.

COPY config config
COPY lib lib
# COPY priv priv # Copied later or if needed for migrations/repo. 
# We need priv if it contains static assets or migrations. Static assets from "benchmarks_data" are external?
# The guide says "If serving static files... COPY benchmarks_data". 
# But usually the runner mounts it or passing ENV.
# The Dockerfile example: `COPY benchmarks_data /app/benchmarks_data`
# I should handle static files in the app logic.

# Get dependencies
RUN mix deps.get --only prod
RUN mix deps.compile

COPY priv priv

# Compile the release
RUN mix compile

# Build the release
RUN mix release

# Runner stage
FROM elixir:1.18-alpine

RUN apk add --no-cache libstdc++ ncurses-libs openssl curl

WORKDIR /app

COPY --from=build /app/_build/prod/rel/wfb_phoenix .
# Also copy benchmarks_data if needed for build, but here we assume it's copied or mounted?
# The guide says:
# "Note: The runner automatically provides benchmarks_data in the build context.
# If building manually with `docker build`, you must copy this folder to the context root or comment this line."
# I will add the COPY command but maybe commented out or conditioned?
# The guide example: `COPY benchmarks_data /app/benchmarks_data`
COPY benchmarks_data /app/benchmarks_data

ENV PORT=8080
ENV MIX_ENV=prod

# Healthcheck
HEALTHCHECK --interval=1s --timeout=3s --retries=30 \
  CMD curl --fail http://localhost:8080/health || exit 1

CMD ["bin/wfb_phoenix", "start"]
