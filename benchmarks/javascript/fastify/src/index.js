const cluster = require('cluster');
const os = require('os');

if (cluster.isPrimary) {
  const numCPUs = os.cpus().length;
  console.log(`Primary ${process.pid} is running`);
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

  const mapToObject = (map) => {
    const obj = Object.create(null);
    for (const [key, value] of map) obj[key] = value;
    return obj;
  };

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
    const results = new Map();
    const categoryStats = new Map();

    for (const order of orders) {
      if (order.status !== 'completed') continue;
      processedOrders++;

      const country = order.country;
      const prevAmount = results.get(country);
      results.set(country, prevAmount === undefined ? order.amount : prevAmount + order.amount);

      const items = order.items;
      for (let i = 0; i < items.length; i++) {
        const item = items[i];
        const category = item.category;
        const prevQty = categoryStats.get(category);
        categoryStats.set(category, prevQty === undefined ? item.quantity : prevQty + item.quantity);
      }
    }

    return {
      processedOrders,
      results: mapToObject(results),
      categoryStats: mapToObject(categoryStats)
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
