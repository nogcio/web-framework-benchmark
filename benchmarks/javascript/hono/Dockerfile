FROM oven/bun:1.3.5-alpine
WORKDIR /app

COPY package.json .
RUN bun install --production

COPY src ./src
COPY benchmarks_data ./benchmarks_data

HEALTHCHECK --interval=1s --timeout=3s --retries=30 CMD wget -qO- http://localhost:8080/health || exit 1

CMD ["bun", "src/index.ts"]
