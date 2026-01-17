import { Elysia } from 'elysia'
import postgres from 'postgres'

const PORT = parseInt(process.env.PORT || '8080')
const DB_HOST = process.env.DB_HOST || 'localhost'
const DB_PORT = parseInt(process.env.DB_PORT || '5432')
const DB_USER = process.env.DB_USER || 'user'
const DB_PASSWORD = process.env.DB_PASSWORD || 'password'
const DB_NAME = process.env.DB_NAME || 'hello_world'
const DB_POOL_SIZE = parseInt(process.env.DB_POOL_SIZE || '256')
const DATA_DIR = process.env.DATA_DIR || 'benchmarks_data'

// Database connection
const sql = postgres({
  host: DB_HOST,
  port: DB_PORT,
  user: DB_USER,
  password: DB_PASSWORD,
  database: DB_NAME,
  max: DB_POOL_SIZE
})

const mapToObject = (map: Map<string, number>) => {
  const obj: Record<string, number> = Object.create(null)
  for (const [key, value] of map) obj[key] = value
  return obj
}

const app = new Elysia()
  // Health check
  .get('/health', async ({ set }) => {
    try {
      await sql`SELECT 1`
      return 'OK'
    } catch (error) {
      set.status = 500
      return 'Database Error'
    }
  })
  
  // Plaintext
  .get('/plaintext', () => 'Hello, World!')
  
  // JSON Aggregate
  .post('/json/aggregate', ({ body }) => {
    const orders = body as any[]
    let processedOrders = 0
    const results = new Map<string, number>()
    const categoryStats = new Map<string, number>()

    for (const order of orders) {
      if (order.status === 'completed') {
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
    }

    return {
      processedOrders,
      results: mapToObject(results),
      categoryStats: mapToObject(categoryStats)
    }
  })
  
  // Database Complex
  .get('/db/user-profile/:email', async ({ params: { email }, set }) => {
    try {
      const [userResult, trendingResult] = await Promise.all([
        sql`SELECT id, username, email, created_at as "createdAt", last_login as "lastLogin", settings FROM users WHERE email = ${email}`,
        sql`SELECT id, title, content, views, created_at as "createdAt" FROM posts ORDER BY views DESC LIMIT 5`
      ])

      if (userResult.length === 0) {
        set.status = 404
        return { error: 'User not found' }
      }

      const user = userResult[0]

      const [updateResult, postsResult] = await Promise.all([
        sql`UPDATE users SET last_login = NOW() WHERE id = ${user.id} RETURNING last_login as "lastLogin"`,
        sql`SELECT id, title, content, views, created_at as "createdAt" FROM posts WHERE user_id = ${user.id} ORDER BY created_at DESC LIMIT 10`
      ])

      user.lastLogin = updateResult[0].lastLogin

      return {
        ...user,
        posts: postsResult,
        trending: trendingResult
      }
    } catch (err) {
      console.error(err)
      set.status = 500
      return { error: 'Internal Server Error' }
    }
  })

app.listen({
  port: PORT,
  reusePort: true
})

console.log(`Worker ${process.pid} listening on port ${PORT}`)
