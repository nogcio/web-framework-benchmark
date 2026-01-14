const fastify = require('fastify');
const { PrismaClient } = require('@prisma/client');
const path = require('path');
const fs = require('fs');
const cluster = require('cluster');
const os = require('os');

if (cluster.isPrimary) {
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
  const app = fastify({
    logger: false,
    disableRequestLogging: true
  });

  // Construct DATABASE_URL if not provided
  if (!process.env.DATABASE_URL) {
    const DB_HOST = process.env.DB_HOST || 'localhost';
    const DB_PORT = process.env.DB_PORT || '5432';
    const DB_USER = process.env.DB_USER || 'user';
    const DB_PASSWORD = process.env.DB_PASSWORD || 'password';
    const DB_NAME = process.env.DB_NAME || 'hello_world';
    // Prisma connection pool configuration can be added to the URL parameters
    // e.g. ?connection_limit=256
    const DB_POOL_SIZE = process.env.DB_POOL_SIZE || '256';
    // Distribute pool across workers
    const numCPUs = os.cpus().length;
    const workerPoolSize = Math.max(1, Math.floor(parseInt(DB_POOL_SIZE) / numCPUs));
    
    process.env.DATABASE_URL = `postgresql://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}?connection_limit=${workerPoolSize}`;
  }

  const prisma = new PrismaClient();
  const DATA_DIR = process.env.DATA_DIR || 'benchmarks_data';

  // Health Check
  app.get('/health', async (request, reply) => {
    try {
      await prisma.$queryRaw`SELECT 1`;
      return 'OK';
    } catch (err) {
      reply.code(500).send({ error: 'Database unavailable' });
    }
  });

  // DB Complex
  app.get('/db/user-profile/:email', async (request, reply) => {
    const email = request.params.email;
    
    try {
      // Parallel: Get User and Trending
      const [user, trending] = await Promise.all([
        prisma.users.findUnique({
          where: { email },
          select: {
            id: true,
            username: true,
            email: true,
            created_at: true,
            last_login: true,
            settings: true
          }
        }),
        prisma.posts.findMany({
          orderBy: { views: 'desc' },
          take: 5,
          select: {
            id: true,
            title: true,
            content: true,
            views: true,
            created_at: true
          }
        })
      ]);

      if (!user) {
        return reply.code(404).send({ error: 'User not found' });
      }

      // Update last_login and Get User Posts
      const [updatedUser, posts] = await Promise.all([
        prisma.users.update({
          where: { id: user.id },
          data: { last_login: new Date() },
          select: { last_login: true }
        }),
        prisma.posts.findMany({
          where: { user_id: user.id },
          orderBy: { created_at: 'desc' },
          take: 10,
          select: {
            id: true,
            title: true,
            content: true,
            views: true,
            created_at: true
          }
        })
      ]);

      // Map to response format
      return {
        id: user.id,
        username: user.username,
        email: user.email,
        createdAt: user.created_at, // Prisma returns Date objects which JSON.stringify handles correctly as ISO strings
        lastLogin: updatedUser.last_login,
        settings: user.settings,
        posts: posts.map(p => ({
          id: p.id,
          title: p.title,
          content: p.content,
          views: p.views,
          createdAt: p.created_at
        })),
        trending: trending.map(p => ({
          id: p.id,
          title: p.title,
          content: p.content,
          views: p.views,
          createdAt: p.created_at
        }))
      };
    } catch (err) {
      console.error(err);
      return reply.code(500).send({ error: 'Internal Server Error' });
    }
  });

  const start = async () => {
    try {
      const port = parseInt(process.env.PORT || '8080');
      await app.listen({ port, host: '0.0.0.0' });
      console.log(`Worker ${process.pid} listening on port ${port}`);
    } catch (err) {
      console.error(err);
      process.exit(1);
    }
  };

  start();
}
