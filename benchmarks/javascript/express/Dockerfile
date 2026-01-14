FROM node:25.2.1-alpine

WORKDIR /app

RUN apk add --no-cache curl

COPY package.json .
RUN npm install --production

COPY src ./src
COPY benchmarks_data /app/benchmarks_data

HEALTHCHECK --interval=1s --timeout=3s --retries=30 CMD curl --fail http://localhost:8080/health || exit 1

CMD ["node", "src/index.js"]
