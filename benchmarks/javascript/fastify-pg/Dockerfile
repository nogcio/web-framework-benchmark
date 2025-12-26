FROM node:25.2.1-alpine3.23

WORKDIR /app

COPY package.json .
RUN npm install --production

COPY src ./src

CMD ["node", "src/index.js"]
