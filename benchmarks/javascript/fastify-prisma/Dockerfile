FROM node:25.2.1-alpine

WORKDIR /app

RUN apk add --no-cache curl openssl

COPY package.json .
RUN npm install

COPY prisma ./prisma
RUN npx prisma generate

COPY src ./src
# We expect benchmarks_data to be mounted or provided
COPY benchmarks_data ./benchmarks_data

ENV PORT=8080
ENV NODE_ENV=production

HEALTHCHECK --interval=1s --timeout=3s --retries=30 CMD curl --fail http://localhost:8080/health || exit 1

CMD ["node", "src/index.js"]
