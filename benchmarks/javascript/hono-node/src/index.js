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
    const results = {}
    const categoryStats = {}

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
  console.log(`Worker ${process.pid} started on port ${port}`)
  
  serve({
    fetch: app.fetch,
    port
  })
}
