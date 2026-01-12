const cluster = require('cluster');
const os = require('os');
const express = require('express');
const path = require('path');

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
  const app = express();
  app.disable('x-powered-by');
  app.disable('etag');

  // Static files
  const staticDir = process.env.DATA_DIR || path.join(process.cwd(), 'benchmarks_data');
  app.use('/files', express.static(staticDir));

  // Health Check
  app.get('/health', (req, res) => {
    res.send('OK');
  });

  // Hello World
  app.get('/', (req, res) => {
    res.type('text/plain').send('Hello, World!');
  });

  app.get('/plaintext', (req, res) => {
    res.type('text/plain').send('Hello, World!');
  });

  app.post('/json/aggregate', express.json({ limit: '50mb' }), (req, res) => {
    const orders = req.body;
    let processed_orders = 0;
    const results = {};
    const category_stats = {};

    if (Array.isArray(orders)) {
      for (const order of orders) {
        if (order.status === 'completed') {
          processed_orders++;

          // results: country -> amount
          results[order.country] = (results[order.country] || 0) + order.amount;

          // category_stats: category -> quantity
          if (Array.isArray(order.items)) {
            for (const item of order.items) {
              category_stats[item.category] = (category_stats[item.category] || 0) + item.quantity;
            }
          }
        }
      }
    }

    res.json({
      processedOrders: processed_orders,
      results,
      categoryStats: category_stats
    });
  });

  const port = parseInt(process.env.PORT || '8080');
  app.listen(port, '0.0.0.0', () => {
    console.log(`Worker ${process.pid} listening on port ${port}`);
  });
}
