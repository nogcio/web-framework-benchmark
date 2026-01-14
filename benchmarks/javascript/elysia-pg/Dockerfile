FROM oven/bun:1.1.42-alpine
WORKDIR /app

COPY package.json .
RUN bun install --production

# Install coreutils for nproc
RUN apk add --no-cache coreutils

COPY src ./src
# We expect benchmarks_data to be mounted or copied in by the runner or manual build
COPY benchmarks_data ./benchmarks_data
COPY entrypoint.sh .
RUN chmod +x entrypoint.sh

ENV PORT=8080

HEALTHCHECK --interval=1s --timeout=3s --retries=30 CMD wget -qO- http://localhost:8080/health || exit 1

CMD ["./entrypoint.sh"]
