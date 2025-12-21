FROM node:25.2.1-slim

WORKDIR /app

COPY package.json .
RUN npm install

COPY src/ src/

CMD ["node", "src/index.js"]
