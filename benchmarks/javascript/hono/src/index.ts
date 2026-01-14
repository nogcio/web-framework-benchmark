import { Hono } from 'hono'
import { serveStatic } from 'hono/bun'
import { cpus } from 'node:os'
import process from 'node:process'
import path from 'node:path'

const numCPUs = cpus().length

if (process.env.BUN_WORKER_ID === undefined) {
  // Master process
  console.log(`Master ${process.pid} is running`)
  console.log(`Spawning ${numCPUs} workers...`)

  const workers = []
  for (let i = 0; i < numCPUs; i++) {
    const worker = Bun.spawn([process.argv[0], ...process.argv.slice(1)], {
      env: {
        ...process.env,
        BUN_WORKER_ID: i.toString(),
      },
      stdout: 'inherit',
      stderr: 'inherit',
      stdin: 'inherit',
    })
    workers.push(worker)
  }

  // Keep master alive
  const wait = () => setTimeout(wait, 100000)
  wait()
} else {
  // Worker process
  const app = new Hono()

  app.get('/health', (c) => c.text('OK'))
  app.get('/', (c) => c.text('Hello, World!'))
  app.get('/plaintext', (c) => c.text('Hello, World!'))

  // Static files
  const dataDir = process.env.DATA_DIR || path.join(process.cwd(), 'benchmarks_data')

  app.on('HEAD', '/files/*', async (c) => {
    const filePath = c.req.path.replace(/^\/files\//, '')
    const file = Bun.file(path.join(dataDir, filePath))
    if (await file.exists()) {
      return new Response(file, {
        status: 200,
        headers: {
          'Content-Type': file.type || 'application/octet-stream'
        }
      })
    }
    return c.notFound()
  })

  app.use('/files/*', serveStatic({
    root: dataDir,
    rewriteRequestPath: (path) => path.replace(/^\/files\//, '')
  }))

  // JSON Aggregation
  app.post('/json/aggregate', async (c) => {
    const orders = await c.req.json()
    let processedOrders = 0
    const results: Record<string, number> = {}
    const categoryStats: Record<string, number> = {}

    if (Array.isArray(orders)) {
      for (const order of orders) {
        if (order.status === 'completed') {
          processedOrders++

          // results: country -> amount
          results[order.country] = (results[order.country] || 0) + order.amount

          // categoryStats: category -> quantity
          if (Array.isArray(order.items)) {
            for (const item of order.items) {
              categoryStats[item.category] = (categoryStats[item.category] || 0) + item.quantity
            }
          }
        }
      }
    }

    return c.json({
      processedOrders,
      results,
      categoryStats
    })
  })

  const port = parseInt(process.env.PORT || '8080')
  
  console.log(`Worker ${process.pid} (ID: ${process.env.BUN_WORKER_ID}) listening on port ${port}`)
  
  Bun.serve({
    port,
    fetch: app.fetch,
    reusePort: true,
  })
}
