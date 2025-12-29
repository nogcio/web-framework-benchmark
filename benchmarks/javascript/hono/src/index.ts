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

  // X-Request-ID middleware
  app.use(async (c, next) => {
    const requestId = c.req.header('x-request-id')
    await next()
    if (requestId) {
      c.header('x-request-id', requestId)
    }
  })

  app.get('/health', (c) => c.text('OK'))
  app.get('/', (c) => c.text('Hello, World!'))
  app.get('/plaintext', (c) => c.text('Hello, World!'))

  // Static files
  const dataDir = process.env.DATA_DIR || path.join(process.cwd(), 'benchmarks_data')
  app.use('/files/*', serveStatic({
    root: dataDir,
    rewriteRequestPath: (path) => path.replace(/^\/files/, '')
  }))

  // JSON
  app.post('/json/:from/:to', async (c) => {
    const from = c.req.param('from')
    const to = c.req.param('to')
    const body = await c.req.json()

    // @ts-ignore
    const servlets = body['web-app']['servlet']
    for (let i = 0; i < servlets.length; i++) {
      if (servlets[i]['servlet-name'] === from) {
        servlets[i]['servlet-name'] = to
      }
    }

    return c.json(body)
  })

  const port = parseInt(process.env.PORT || '8080')
  
  console.log(`Worker ${process.pid} (ID: ${process.env.BUN_WORKER_ID}) listening on port ${port}`)
  
  Bun.serve({
    port,
    fetch: app.fetch,
    reusePort: true,
  })
}
