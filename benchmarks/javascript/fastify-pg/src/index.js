const cluster = require('cluster');
const os = require('os');

if (cluster.isMaster) {
  const numCPUs = os.cpus().length;
  console.log(`Master ${process.pid} is running`);
  console.log(`Forking ${numCPUs} workers...`);
  const poolSize = Math.max(1, Math.floor(256 / numCPUs));

  for (let i = 0; i < numCPUs; i++) {
    cluster.fork({ DB_POOL_SIZE: poolSize });
  }

  cluster.on('exit', (worker, code, signal) => {
    console.log(`worker ${worker.process.pid} died`);
    cluster.fork(); // Restart worker on death
  });
} else {
  const fastify = require('fastify')({
    logger: false,
    disableRequestLogging: true
  });
  const pool = require('./db');
  const crypto = require('crypto');
  const jwt = require('jsonwebtoken');

  const JWT_SECRET = 'benchmark-secret';

  // Health Check
  fastify.get('/health', async (request, reply) => {
    try {
      await pool.query('SELECT 1');
      return 'OK';
    } catch (err) {
      reply.code(500).send({ error: 'Database unavailable' });
    }
  });

  // Database Tests
  fastify.get('/db/read/one', async (request, reply) => {
    const id = parseInt(request.query.id);
    const res = await pool.query({
      name: 'db_read_one',
      text: 'SELECT id, name, created_at as "createdAt", updated_at as "updatedAt" FROM hello_world WHERE id = $1',
      values: [id]
    });
    return res.rows[0];
  });

  fastify.get('/db/read/many', async (request, reply) => {
    const offset = parseInt(request.query.offset) || 0;
    const limit = parseInt(request.query.limit) || 50;
    const res = await pool.query({
      name: 'db_read_many',
      text: 'SELECT id, name, created_at as "createdAt", updated_at as "updatedAt" FROM hello_world ORDER BY id LIMIT $1 OFFSET $2',
      values: [limit, offset]
    });
    return res.rows;
  });

  fastify.post('/db/write/insert', async (request, reply) => {
    const { name } = request.body;
    const now = new Date();
    const res = await pool.query({
      name: 'db_write_insert',
      text: 'INSERT INTO hello_world (name, created_at, updated_at) VALUES ($1, $2, $3) RETURNING id, name, created_at as "createdAt", updated_at as "updatedAt"',
      values: [name, now, now]
    });
    return res.rows[0];
  });

  // Tweet Service

  // Auth Middleware
  const authenticate = async (request, reply) => {
    try {
      const authHeader = request.headers.authorization;
      if (!authHeader) {
        return reply.code(401).send({ error: 'Unauthorized' });
      }
      const token = authHeader.split(' ')[1];
      const decoded = jwt.verify(token, JWT_SECRET);
      request.user = decoded;
    } catch (err) {
      return reply.code(401).send({ error: 'Unauthorized' });
    }
  };

  fastify.post('/api/auth/register', async (request, reply) => {
    const { username, password } = request.body;
    const passwordHash = crypto.createHash('sha256').update(password).digest('hex');
    
    try {
      await pool.query('INSERT INTO users (username, password_hash) VALUES ($1, $2)', [username, passwordHash]);
      reply.code(201).send();
    } catch (err) {
      if (err.code === '23505') { // Unique violation
        // Treat duplicate registration as success to keep the benchmark workload stable.
        reply.code(201).send();
      } else {
        throw err;
      }
    }
  });

  fastify.post('/api/auth/login', async (request, reply) => {
    const { username, password } = request.body;
    const passwordHash = crypto.createHash('sha256').update(password).digest('hex');
    
    const res = await pool.query('SELECT id FROM users WHERE username = $1 AND password_hash = $2', [username, passwordHash]);
    
    if (res.rows.length === 0) {
      reply.code(401).send({ error: 'Invalid credentials' });
      return;
    }
    
    const user = res.rows[0];
    const token = jwt.sign({ sub: user.id, name: username }, JWT_SECRET);
    return { token };
  });

  fastify.get('/api/feed', { preHandler: authenticate }, async (request, reply) => {
    const res = await pool.query(`
      SELECT t.id, u.username, t.content, t.created_at as "createdAt", (SELECT COUNT(*) FROM likes l WHERE l.tweet_id = t.id) as likes
      FROM tweets t
      JOIN users u ON t.user_id = u.id
      ORDER BY t.created_at DESC
      LIMIT 20
    `);
    // Convert likes to int because COUNT returns string in pg
    const feed = res.rows.map(row => ({
      ...row,
      likes: parseInt(row.likes)
    }));
    return feed;
  });

  fastify.get('/api/tweets/:id', { preHandler: authenticate }, async (request, reply) => {
    const id = parseInt(request.params.id);
    const res = await pool.query(`
      SELECT t.id, u.username, t.content, t.created_at as "createdAt", (SELECT COUNT(*) FROM likes l WHERE l.tweet_id = t.id) as likes
      FROM tweets t
      JOIN users u ON t.user_id = u.id
      WHERE t.id = $1
    `, [id]);
    
    if (res.rows.length === 0) {
      reply.code(404).send({ error: 'Tweet not found' });
      return;
    }
    
    const tweet = res.rows[0];
    tweet.likes = parseInt(tweet.likes);
    return tweet;
  });

  fastify.post('/api/tweets', { preHandler: authenticate }, async (request, reply) => {
    const { content } = request.body;
    const userId = request.user.sub;
    
    await pool.query(
      'INSERT INTO tweets (user_id, content) VALUES ($1, $2)',
      [userId, content]
    );
    
    reply.code(201).send();
  });

  fastify.post('/api/tweets/:id/like', { preHandler: authenticate }, async (request, reply) => {
    const tweetId = parseInt(request.params.id);
    const userId = request.user.sub;
    
    const res = await pool.query('DELETE FROM likes WHERE user_id = $1 AND tweet_id = $2', [userId, tweetId]);
    
    if (res.rowCount === 0) {
      try {
        await pool.query('INSERT INTO likes (user_id, tweet_id) VALUES ($1, $2)', [userId, tweetId]);
      } catch (err) {
        // Ignore if tweet doesn't exist or race condition
      }
    }
    
    reply.send();
  });

  const start = async () => {
    try {
      const port = parseInt(process.env.PORT || '8000');
      await fastify.listen({ port, host: '0.0.0.0' });
      console.log(`Worker ${process.pid} listening on port ${port}`);
    } catch (err) {
      console.error(err);
      process.exit(1);
    }
  };

  start();
}
