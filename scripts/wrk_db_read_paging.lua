-- scripts/wrk_db_read_paging.lua
-- Request /db/read/many?offset=N&limit=50 and validate count and min/max id.

local max_id = 1000
local limit = 50
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

function request()
  local max_offset = math.max(0, max_id - 1)
  local offset = math.random(0, max_offset)
  seq = seq + 1
  local thread_id = 0
  if wrk.thread and wrk.thread.id then
    thread_id = tonumber(wrk.thread.id) or 0
  end
  local reqid = string.format("%d-%d", thread_id, seq)
  pending[reqid] = { offset = offset }
  local path = string.format("/db/read/many?offset=%d&limit=%d", offset, limit)
  local hdrs = { ["x-request-id"] = reqid }
  return wrk.format("GET", path, hdrs)
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
  local expected = nil
  if reqid then
    expected = pending[reqid]
    pending[reqid] = nil
  end
  if not expected then
    errors = (errors or 0) + 1
    return
  end
  local ids = {}
  for idstr in body:gmatch('"id"%s*:%s*([%d]+)') do
    table.insert(ids, tonumber(idstr))
  end
  local count = #ids
  local expected_count = math.min(limit, math.max(0, max_id - expected.offset))
  if count ~= expected_count then
    errors = (errors or 0) + 1
    return
  end
  if count > 0 then
    local min_id = ids[1]
    local max_id_seen = ids[#ids]
    if min_id ~= (expected.offset + 1) or max_id_seen ~= (expected.offset + count) then
      errors = (errors or 0) + 1
      return
    end
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
