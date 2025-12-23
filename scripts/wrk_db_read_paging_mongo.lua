-- scripts/wrk_db_read_paging_mongo.lua
-- Request /db/read/many?offset=N&limit=50 and validate count and min/max id.
-- For MongoDB with ObjectIds, we still use offset/limit pagination in this benchmark.

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

  -- Count items in JSON array by counting "id" occurrences
  local count = 0
  for _ in body:gmatch('"id"') do 
    count = count + 1 
  end

  local expected_count = math.min(limit, math.max(0, max_id - expected.offset))
  if count ~= expected_count then
    errors = (errors or 0) + 1
    return
  end
  
  -- We can't easily validate min/max ID because they are ObjectIds and not sequential integers in the response body (they are hex strings).
  -- However, since we generated them deterministically from 1..1000, they ARE sequential in terms of creation order, 
  -- but the hex string comparison might be tricky if we don't convert back.
  -- But the original script validates min_id and max_id_seen.
  -- Let's try to extract the first and last ID and verify them if possible.
  -- The IDs are "0000...hex(i)".
  
  if count > 0 then
    local ids = {}
    for idstr in body:gmatch('"id"%s*:%s*"([^"]+)"') do
      table.insert(ids, idstr)
    end
    
    if #ids ~= count then
       -- Something is wrong with parsing
       errors = (errors or 0) + 1
       return
    end

    -- Helper to convert hex ObjectId back to int
    local function oid_to_int(oid)
       return tonumber(oid, 16)
    end

    local first_id = oid_to_int(ids[1])
    local last_id = oid_to_int(ids[#ids])
    
    if first_id ~= (expected.offset + 1) or last_id ~= (expected.offset + count) then
      errors = (errors or 0) + 1
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
