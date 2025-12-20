-- scripts/wrk_json.lua
-- Generate requests to /json/{random_1}/{random_2} and validate response JSON
-- scripts/wrk_json.lua
-- Generate requests to /json/{random_1}/{random_2} and validate JSON response.
-- NOTE: This script keeps a per-thread FIFO queue of expected values and assumes
-- responses are received in the same order as requests for that thread. That
-- holds for typical wrk usage (no pipelining across multiple outstanding
-- requests on the same thread). If you use aggressive pipelining/connections,
-- consider encoding an identifier in the request that the server echoes back.

local max_id = 100000

local counter = 1
local threads = {}

function setup(thread)
   thread:set("id", counter)
   table.insert(threads, thread)
   counter = counter + 1
end

function init(args)
  math.randomseed(os.time())
  pending = {}
  errors = 0
  seq = 0
end

local function parse_fields(body)
  -- tolerant numeric parse for fields: allows spaces, optional sign
  local f1 = body:match('"field1"%s*:%s*([-]?%d+)')
  local f2 = body:match('"field2"%s*:%s*([-]?%d+)')
  if not f1 or not f2 then
    return nil, nil
  end
  return tonumber(f1), tonumber(f2)
end

function request()
  local r1 = math.random(1, max_id)
  local r2 = math.random(1, max_id)
  seq = seq + 1
  local thread_id = 0
  if wrk.thread and wrk.thread.id then
    thread_id = tonumber(wrk.thread.id) or 0
  end
  local reqid = string.format("%d-%d", thread_id, seq)
  -- store expected by request id
  pending[reqid] = { r1 = r1, r2 = r2 }
  local path = string.format("/json/%d/%d", r1, r2)
  local hdrs = { ["x-request-id"] = reqid }
  return wrk.format("GET", path, hdrs)
end

function response(status, headers, body)
  if status ~= 200 then
    errors = errors + 1
    return
  end
  -- try to obtain request id from response headers first
  local reqid = nil
  if headers then
    reqid = headers["x-request-id"] or headers["X-Request-Id"] or headers["X-Request-ID"]
  end
  -- fall back to JSON body field "request_id" (string)
  if not reqid then
    reqid = body:match('"request_id"%s*:%s*"([^"]+)"')
  end
  local expected = nil
  if reqid then
    expected = pending[reqid]
    pending[reqid] = nil
  else
    -- if no reqid found, we cannot match — count as error
    errors = errors + 1
    return
  end
  if not expected then
    errors = errors + 1
    return
  end
  local f1, f2 = parse_fields(tostring(body or ""))
  if not f1 or not f2 then
    errors = errors + 1
    return
  end
  if f1 ~= expected.r1 or f2 ~= expected.r2 then
    errors = errors + 1
  end
end

function done(summary, latency, requests)
   local total_errors = 0
   for index, thread in ipairs(threads) do
      local errors    = thread:get("errors")
      total_errors = total_errors + errors
   end
   print("Errors: " .. total_errors)
end