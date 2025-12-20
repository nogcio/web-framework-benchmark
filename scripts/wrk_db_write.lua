-- scripts/wrk_db_write.lua
-- Insert a row via /db/write/insert?name=N and validate returned JSON contains the name

local counter = 1
local threads = {}

function setup(thread)
  thread:set("id", counter)
  table.insert(threads, thread)
  counter = counter + 1
end

function init(args)
  math.randomseed(os.time())
  seq = 0
  errors = 0
end

local function rand_name()
  return string.format("write_%d_%d", math.random(1, 1000000), os.time())
end

function request()
  seq = seq + 1
  local name = rand_name()
  local thread_id = 0
    if wrk.thread and wrk.thread.id then
    thread_id = tonumber(wrk.thread.id) or 0
  end
    local reqid = string.format("%d-%d", thread_id, seq)
    -- send name as JSON in the POST body
    local path = "/db/write/insert"
    local body = string.format('{"name":"%s"}', name)
    local hdrs = {
      ["x-request-id"] = reqid,
      ["content-type"] = "application/json"
    }
    -- store expected name in headers table so response() can validate by matching reqid->name mapping
  pending = pending or {}
  pending[reqid] = name
    return wrk.format("POST", path, hdrs, body)
end

function response(status, headers, body)
  if status ~= 200 then
    errors = (errors or 0) + 1
    return
  end
  local reqid = nil
  if headers then
    reqid = headers["x-request-id"] or headers["X-Request-Id"] or headers["X-Request-ID"]
  end
  if not reqid then
    reqid = body:match('"request_id"%s*:%s*"([^"]+)"')
  end
  local expected_name = nil
  if reqid then
    expected_name = pending[reqid]
    pending[reqid] = nil
  end
  if not expected_name then
    errors = (errors or 0) + 1
    return
  end
  local name = body:match('"name"%s*:%s*"([^"]+)"')
  if not name then
    errors = (errors or 0) + 1
    return
  end
  if name ~= expected_name then
    errors = (errors or 0) + 1
  end
end

function done(summary, latency, requests)
  local total_errors = 0
  for index, thread in ipairs(threads) do
    local e = thread:get("errors") or 0
    total_errors = total_errors + e
  end
  print("Errors: " .. total_errors)
end
