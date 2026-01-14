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

  // JSON Aggregation schema
  const jsonAggregateSchema = {
    body: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          status: { type: 'string' },
          amount: { type: 'integer' },
          country: { type: 'string' },
          items: {
            type: 'array',
            items: {
              type: 'object',
              properties: {
                quantity: { type: 'integer' },
                category: { type: 'string' }
              }
            }
          }
        },
        required: ['status'] // minimal requirement for logic
      }
    },
    response: {
      200: {
        type: 'object',
        properties: {
          processedOrders: { type: 'integer' },
          results: {
            type: 'object',
            additionalProperties: { type: 'integer' }
          },
          categoryStats: {
            type: 'object',
            additionalProperties: { type: 'integer' }
          }
        }
      }
    }
  };

  // JSON Aggregation
  fastify.post('/json/aggregate', { schema: jsonAggregateSchema }, async (request, reply) => {
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
