FROM node:25.2.1-alpine3.23

WORKDIR /app

RUN apk add --no-cache curl

COPY package.json .
RUN npm install --production

COPY src ./src

HEALTHCHECK --interval=5s --timeout=3s --retries=3 CMD curl --fail http://localhost:8080/health || exit 1

CMD ["node", "src/index.js"]
