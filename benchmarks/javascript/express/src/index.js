const cluster = require('cluster');
const os = require('os');
const express = require('express');
const path = require('path');

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
  const app = express();
  app.disable('x-powered-by');
  app.disable('etag');

  const mapToObject = (map) => {
    const obj = Object.create(null);
    for (const [key, value] of map) obj[key] = value;
    return obj;
  };

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

    res.json({
      processedOrders,
      results: mapToObject(results),
      categoryStats: mapToObject(categoryStats)
    });
  });

  const port = parseInt(process.env.PORT || '8080');
  app.listen(port, '0.0.0.0', () => {
    console.log(`Worker ${process.pid} listening on port ${port}`);
  });
}
