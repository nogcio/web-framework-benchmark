FROM elixir:1.18-slim AS build
RUN apt-get update && apt-get install -y protobuf-compiler git build-essential

WORKDIR /app

# Install Hex + Rebar
RUN mix local.hex --force && \
    mix local.rebar --force

# Install protobuf plugin
RUN mix escript.install hex protobuf --force
ENV PATH="/root/.mix/escripts:$PATH"


COPY mix.exs ./
COPY config config

RUN mix deps.get
RUN mix deps.compile

COPY proto proto
# Generate code
RUN mkdir -p lib
RUN protoc --elixir_out=plugins=grpc:./lib -I proto proto/analytics.proto proto/health.proto

COPY lib lib

ENV MIX_ENV=prod
RUN mix release

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y libssl-dev openssl libtinfo6 locales curl && apt-get clean
RUN echo "en_US.UTF-8 UTF-8" > /etc/locale.gen && locale-gen
ENV LANG en_US.UTF-8
ENV ERL_FLAGS="+K true +sbwt very_long +swt very_low +S 4"

# Install grpc_health_probe
COPY --from=ghcr.io/grpc-ecosystem/grpc-health-probe:v0.4.24 /ko-app/grpc-health-probe /bin/grpc_health_probe

WORKDIR /app
COPY --from=build /app/_build/prod/rel/grpc_benchmark .

HEALTHCHECK --interval=1s --timeout=3s --retries=30 \
    CMD /bin/grpc_health_probe -addr=:8080 || exit 1

CMD ["/app/bin/grpc_benchmark", "start"]
