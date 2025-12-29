FROM oven/bun:1.3.5-alpine
WORKDIR /app

COPY package.json .
RUN bun install --production

COPY src ./src
COPY benchmarks_data ./benchmarks_data

HEALTHCHECK --interval=5s --timeout=3s --retries=3 CMD wget -qO- http://localhost:8080/health || exit 1

CMD ["bun", "src/index.ts"]
