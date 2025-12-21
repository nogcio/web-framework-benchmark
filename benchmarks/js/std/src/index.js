const http = require('http');
const url = require('url');
const fs = require('fs');
const path = require('path');
const { Pool } = require('pg');

// Configuration
const PORT = process.env.PORT || 8000;
const DB_HOST = process.env.DB_HOST || 'db';
const DB_PORT = process.env.DB_PORT || 5432;
const DB_USER = process.env.DB_USER || 'benchmark';
const DB_PASSWORD = process.env.DB_PASSWORD || 'benchmark';
const DB_NAME = process.env.DB_NAME || 'benchmark';
const DATA_DIR = process.env.DATA_DIR || 'benchmarks_data';

// Database connection
const pool = new Pool({
  host: DB_HOST,
  port: DB_PORT,
  user: DB_USER,
  password: DB_PASSWORD,
  database: DB_NAME,
  max: 20, // Adjust based on load
});

// Helper to send JSON response
const sendJSON = (res, data, status = 200, reqId) => {
  res.writeHead(status, {
    'Content-Type': 'application/json',
    ...(reqId && { 'x-request-id': reqId }),
  });
  res.end(JSON.stringify(data));
};

// Helper to send Text response
const sendText = (res, data, status = 200, reqId) => {
  res.writeHead(status, {
    'Content-Type': 'text/plain',
    ...(reqId && { 'x-request-id': reqId }),
  });
  res.end(data);
};

// Helper to parse JSON body
const parseBody = (req) => {
  return new Promise((resolve, reject) => {
    let body = '';
    req.on('data', chunk => {
      body += chunk.toString();
    });
    req.on('end', () => {
      try {
        resolve(JSON.parse(body));
      } catch (e) {
        reject(e);
      }
    });
    req.on('error', reject);
  });
};

const server = http.createServer(async (req, res) => {
  const parsedUrl = url.parse(req.url, true);
  const pathname = parsedUrl.pathname;
  const method = req.method;
  const reqId = req.headers['x-request-id'];

  try {
    // 5.1. Root / Hello World
    if (method === 'GET' && pathname === '/') {
      return sendText(res, 'Hello, World!', 200, reqId);
    }

    // 5.2. Health Check
    if (method === 'GET' && pathname === '/health') {
      try {
        await pool.query('SELECT 1');
        return sendText(res, 'OK', 200, reqId);
      } catch (e) {
        return sendText(res, 'Service Unavailable', 503, reqId);
      }
    }

    // 5.3. Info
    if (method === 'GET' && pathname === '/info') {
      return sendText(res, '25.2.1,hello_world,json,db_read_one,db_read_paging,db_write,static_files', 200, reqId);
    }

    // 5.4. JSON Processing
    // Path: POST /json/{from}/{to}
    if (method === 'POST' && pathname.startsWith('/json/')) {
      const parts = pathname.split('/');
      if (parts.length === 4) {
        const from = parts[2];
        const to = parts[3];
        
        try {
          const body = await parseBody(req);
          
          const traverse = (obj) => {
            if (Array.isArray(obj)) {
              obj.forEach(traverse);
            } else if (typeof obj === 'object' && obj !== null) {
              for (const key in obj) {
                if (key === 'servlet-name' && obj[key] === from) {
                  obj[key] = to;
                }
                traverse(obj[key]);
              }
            }
          };

          traverse(body);
          return sendJSON(res, body, 200, reqId);
        } catch (e) {
          return sendText(res, 'Bad Request', 400, reqId);
        }
      }
    }

    // 5.5. Database: Read One
    if (method === 'GET' && pathname === '/db/read/one') {
      const id = parsedUrl.query.id;
      if (!id) {
        return sendText(res, 'Missing id', 400, reqId);
      }
      
      const result = await pool.query('SELECT * FROM hello_world WHERE id = $1', [id]);
      if (result.rows.length === 0) {
        return sendText(res, 'Not Found', 404, reqId);
      }
      return sendJSON(res, result.rows[0], 200, reqId);
    }

    // 5.6. Database: Read Many (Paging)
    if (method === 'GET' && pathname === '/db/read/many') {
      const offset = parsedUrl.query.offset;
      const limit = parsedUrl.query.limit || 50;
      
      if (!offset) {
        return sendText(res, 'Missing offset', 400, reqId);
      }

      const result = await pool.query('SELECT * FROM hello_world ORDER BY id LIMIT $1 OFFSET $2', [limit, offset]);
      return sendJSON(res, result.rows, 200, reqId);
    }

    // 5.7. Database: Write (Insert)
    if (method === 'POST' && pathname === '/db/write/insert') {
      let name;
      
      // Check query param first
      if (parsedUrl.query.name) {
        name = parsedUrl.query.name;
      } else {
        // Try body
        try {
          const body = await parseBody(req);
          name = body.name;
        } catch (e) {
          // Ignore body parse error if name not found there
        }
      }

      if (!name) {
        return sendText(res, 'Missing name', 400, reqId);
      }

      const result = await pool.query(
        'INSERT INTO hello_world (name, created_at, updated_at) VALUES ($1, NOW(), NOW()) RETURNING *',
        [name]
      );
      return sendJSON(res, result.rows[0], 200, reqId);
    }

    // 5.8. Static Files
    if (method === 'GET' && pathname.startsWith('/files/')) {
      const filename = pathname.substring('/files/'.length);
      
      // Security check
      if (filename.includes('..') || filename.includes('/')) {
        return sendText(res, 'Forbidden', 403, reqId);
      }

      const filePath = path.join(DATA_DIR, filename);
      
      fs.readFile(filePath, (err, data) => {
        if (err) {
          if (err.code === 'ENOENT') {
            return sendText(res, 'Not Found', 404, reqId);
          }
          return sendText(res, 'Internal Server Error', 500, reqId);
        }
        
        res.writeHead(200, {
          'Content-Type': 'application/octet-stream',
          ...(reqId && { 'x-request-id': reqId }),
        });
        res.end(data);
      });
      return;
    }

    // 404
    sendText(res, 'Not Found', 404, reqId);

  } catch (e) {
    console.error(e);
    sendText(res, 'Internal Server Error', 500, reqId);
  }
});

server.listen(PORT, () => {
  console.log(`Server listening on port ${PORT}`);
});
