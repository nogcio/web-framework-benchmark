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

  // Health Check
  fastify.get('/health', async (request, reply) => {
    return 'OK';
  });

  // Hello World
  fastify.get('/', async (request, reply) => {
    return 'Hello, World!';
  });

  fastify.get('/plaintext', async (request, reply) => {
    return 'Hello, World!';
  });

  // JSON Aggregation
  fastify.post('/json/aggregate', async (request, reply) => {
    const orders = request.body;
    let processedOrders = 0;
    const results = {};
    const categoryStats = {};

    if (Array.isArray(orders)) {
      for (const order of orders) {
        if (order.status === 'completed') {
          processedOrders++;

          // results: country -> amount
          results[order.country] = (results[order.country] || 0) + order.amount;

          // categoryStats: category -> quantity
          if (Array.isArray(order.items)) {
            for (const item of order.items) {
              categoryStats[item.category] = (categoryStats[item.category] || 0) + item.quantity;
            }
          }
        }
      }
    }

    return {
      processedOrders,
      results,
      categoryStats
    };
  });

  const start = async () => {
    try {
      const port = parseInt(process.env.PORT || '8080');
      await fastify.listen({ port, host: '0.0.0.0' });
      console.log(`Worker ${process.pid} listening on port ${port}`);
    } catch (err) {
      console.error(err);
      process.exit(1);
    }
  };

  start();
}
