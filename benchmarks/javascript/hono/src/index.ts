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

  const mapToObject = (map: Map<string, number>) => {
    const obj: Record<string, number> = Object.create(null)
    for (const [key, value] of map) obj[key] = value
    return obj
  }

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
    const results = new Map<string, number>()
    const categoryStats = new Map<string, number>()

    for (const order of orders) {
      if (order.status !== 'completed') continue
      processedOrders++

      const country = order.country
      const prevAmount = results.get(country)
      results.set(country, prevAmount === undefined ? order.amount : prevAmount + order.amount)

      const items = order.items
      for (let i = 0; i < items.length; i++) {
        const item = items[i]
        const category = item.category
        const prevQty = categoryStats.get(category)
        categoryStats.set(category, prevQty === undefined ? item.quantity : prevQty + item.quantity)
      }
    }

    return c.json({
      processedOrders,
      results: mapToObject(results),
      categoryStats: mapToObject(categoryStats)
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
