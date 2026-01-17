import { serve } from '@hono/node-server'
import { Hono } from 'hono'
import cluster from 'node:cluster'
import { cpus } from 'node:os'
import process from 'node:process'
import fs from 'node:fs'
import path from 'node:path'
import { Readable } from 'node:stream'

const numCPUs = cpus().length

if (cluster.isPrimary) {
  console.log(`Primary ${process.pid} is running`)
  console.log(`Forking ${numCPUs} workers...`)

  for (let i = 0; i < numCPUs; i++) {
    cluster.fork()
  }

  cluster.on('exit', (worker, code, signal) => {
    console.log(`worker ${worker.process.pid} died`)
  })
} else {
  const app = new Hono()

  const mapToObject = (map) => {
    const obj = Object.create(null)
    for (const [key, value] of map) obj[key] = value
    return obj
  }

  app.get('/health', (c) => c.text('OK'))
  app.get('/', (c) => c.text('Hello, World!'))
  app.get('/plaintext', (c) => c.text('Hello, World!'))

  const dataDir = process.env.DATA_DIR || path.join(process.cwd(), 'benchmarks_data')

  // Static files handling matching the logic
  const serveFile = async (c, method) => {
    const filePath = c.req.path.replace(/^\/files\//, '')
    // prevent directory traversal
    if (filePath.includes('..')) {
        return c.notFound()
    }
    const fullPath = path.join(dataDir, filePath)

    try {
        if (!fs.existsSync(fullPath)) {
            return c.notFound()
        }
        
        const stat = fs.statSync(fullPath)
        
        c.header('Content-Type', 'application/octet-stream')
        c.header('Content-Length', stat.size.toString())
        
        if (method === 'HEAD') {
            return c.body(null)
        }

        const fileStream = fs.createReadStream(fullPath)
        return c.body(Readable.toWeb(fileStream))
    } catch (e) {
        return c.notFound()
    }
  }

  app.on(['GET', 'HEAD'], '/files/*', (c) => serveFile(c, c.req.method))

  // JSON Aggregation
  app.post('/json/aggregate', async (c) => {
    const orders = await c.req.json()
    let processedOrders = 0
    const results = new Map()
    const categoryStats = new Map()

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
  console.log(`Worker ${process.pid} started on port ${port}`)
  
  serve({
    fetch: app.fetch,
    port
  })
}
