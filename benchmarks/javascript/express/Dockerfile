FROM node:25.2.1-alpine

WORKDIR /app

COPY package.json .
RUN npm install --production

COPY src ./src
COPY benchmarks_data /app/benchmarks_data

CMD ["node", "src/index.js"]
