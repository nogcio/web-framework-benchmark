const { Pool } = require('pg');

const poolSize = parseInt(process.env.DB_POOL_SIZE || '10');

const pool = new Pool({
  host: process.env.DB_HOST || 'localhost',
  port: parseInt(process.env.DB_PORT || '5432'),
  database: process.env.DB_NAME || 'benchmark',
  user: process.env.DB_USER || 'benchmark',
  password: process.env.DB_PASSWORD || 'benchmark',
  max: poolSize,
  min: poolSize
});

module.exports = pool;
