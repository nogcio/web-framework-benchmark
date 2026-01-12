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

  // Health Check
  fastify.get('/health', async (request, reply) => {
    try {
      await pool.query('SELECT 1');
      return 'OK';
    } catch (err) {
      reply.code(500).send({ error: 'Database unavailable' });
    }
  });

  const getUserProfile = async (email) => {
    // Parallel: Get User and Trending
    const [userResult, trendingResult] = await Promise.all([
      pool.query('SELECT id, username, email, created_at as "createdAt", last_login as "lastLogin", settings FROM users WHERE email = $1', [email]),
      pool.query('SELECT id, title, content, views, created_at as "createdAt" FROM posts ORDER BY views DESC LIMIT 5')
    ]);

    if (userResult.rows.length === 0) {
      return null;
    }

    const user = userResult.rows[0];
    
    // Update last_login and Get User Posts
    const [updateResult, postsResult] = await Promise.all([
      pool.query('UPDATE users SET last_login = NOW() WHERE id = $1 RETURNING last_login as "lastLogin"', [user.id]),
      pool.query(
        'SELECT id, title, content, views, created_at as "createdAt" FROM posts WHERE user_id = $1 ORDER BY created_at DESC LIMIT 10',
        [user.id]
      )
    ]);

    user.lastLogin = updateResult.rows[0].lastLogin;

    return {
      ...user,
      posts: postsResult.rows,
      trending: trendingResult.rows
    };
  };

  // DB Complex Read
  fastify.get('/db/user-profile/:email', async (request, reply) => {
    const email = request.params.email;
    
    try {
      const result = await getUserProfile(email);
      if (!result) {
        return reply.code(404).send({ error: 'User not found' });
      }
      return result;
    } catch (err) {
      console.error(err);
      return reply.code(500).send({ error: 'Internal Server Error' });
    }
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
