FROM oven/bun:1.3.5-alpine
WORKDIR /app

COPY package.json .
RUN bun install --production

COPY src ./src
COPY benchmarks_data ./benchmarks_data

CMD ["bun", "src/index.ts"]
