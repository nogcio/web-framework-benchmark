-- scripts/wrk_db_read_one_mongo.lua
-- Generate requests to /db/read/one?id=ObjectId and validate the JSON response

local max_id = 1000
local counter = 1
local threads = {}

function setup(thread)
  thread:set("id", counter)
  table.insert(threads, thread)
  counter = counter + 1
end

function init(args)
  math.randomseed(os.time() + math.floor(os.clock() * 100000))
  pending = {}
  seq = 0
  errors = 0
end

function get_object_id(i)
  local hex = string.format("%x", i)
  while #hex < 24 do
    hex = "0" .. hex
  end
  return hex
end

function request()
  local id_int = math.random(1, max_id)
  local id = get_object_id(id_int)
  seq = seq + 1
  local thread_id = 0
  if wrk.thread and wrk.thread.id then
    thread_id = tonumber(wrk.thread.id) or 0
  end
  local reqid = string.format("%d-%d", thread_id, seq)
  pending[reqid] = { id = id, name = "name_" .. id_int }
  local path = string.format("/db/read/one?id=%s", id)
  local hdrs = { ["x-request-id"] = reqid }
  return wrk.format("GET", path, hdrs)
end

function response(status, headers, body)
  if status ~= 200 then
    return
  end
  local reqid = nil
  if headers then
    reqid = headers["x-request-id"] or headers["X-Request-Id"] or headers["X-Request-ID"]
  end
  if not reqid then
    reqid = body:match('"request_id"%s*:%s*"([^"]+)"')
  end
  local expected = nil
  if reqid then
    expected = pending[reqid]
    pending[reqid] = nil
  end
  
  if not expected then
    errors = (errors or 0) + 1
    return
  end

  -- Match ObjectId string
  local id = body:match('"id"%s*:%s*"([^"]+)"')
  local name = body:match('"name"%s*:%s*"([^"]+)"')

  if not id or not name then
    errors = (errors or 0) + 1
    return
  end

  -- Validate ID matches expected ObjectId
  if id ~= expected.id then
    errors = (errors or 0) + 1
    return
  end

  if name ~= expected.name then
    errors = (errors or 0) + 1
  end
end

function done(summary, latency, requests)
  local total_errors = 0
  for _, thread in ipairs(threads) do
    local thread_errors = thread:get("errors") or 0
    total_errors = total_errors + thread_errors
  end
  print("Errors: " .. total_errors)
end
