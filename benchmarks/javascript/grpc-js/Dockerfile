FROM node:25.2.1-alpine

WORKDIR /app

COPY package.json .
RUN npm install --production

COPY src ./src
COPY proto ./proto

COPY --from=ghcr.io/grpc-ecosystem/grpc-health-probe:v0.4.24 /ko-app/grpc-health-probe /bin/grpc_health_probe

HEALTHCHECK --interval=1s --timeout=3s --retries=30 CMD ["/bin/grpc_health_probe", "-addr=127.0.0.1:8080"]

CMD ["node", "src/index.js"]
