const cluster = require('cluster');
const os = require('os');

if (cluster.isMaster) {
  const numCPUs = os.cpus().length;
  console.log(`Master ${process.pid} is running`);
  console.log(`Forking ${numCPUs} workers...`);

  for (let i = 0; i < numCPUs; i++) {
    cluster.fork();
  }

  cluster.on('exit', (worker, code, signal) => {
    console.log(`worker ${worker.process.pid} died`);
    cluster.fork();
  });
} else {
  const fastify = require('fastify')({
    logger: false,
    disableRequestLogging: true
  });
  const path = require('path');

  // Register static file serving
  fastify.register(require('@fastify/static'), {
    root: process.env.DATA_DIR || path.join(process.cwd(), 'benchmarks_data'),
    prefix: '/files/', // optional: default '/'
  });

  // X-Request-ID handling
  fastify.addHook('onSend', (request, reply, payload, done) => {
    const requestId = request.headers['x-request-id'];
    if (requestId) {
      reply.header('x-request-id', requestId);
    }
    done();
  });

  // Health Check
  fastify.get('/health', async (request, reply) => {
    return 'OK';
  });

  // Hello World
  fastify.get('/', async (request, reply) => {
    return 'Hello, World!';
  });

  // JSON Serialization
  fastify.post('/json/:from/:to', async (request, reply) => {
    const { from, to } = request.params;
    const body = request.body;

    const servlets = body['web-app']['servlet'];
    for (let i = 0; i < servlets.length; i++) {
      if (servlets[i]['servlet-name'] === from) {
        servlets[i]['servlet-name'] = to;
      }
    }

    return body;
  });

  const start = async () => {
    try {
      const port = parseInt(process.env.PORT || '8000');
      await fastify.listen({ port, host: '0.0.0.0' });
      console.log(`Worker ${process.pid} listening on port ${port}`);
    } catch (err) {
      console.error(err);
      process.exit(1);
    }
  };

  start();
}
