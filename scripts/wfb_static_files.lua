local http = require("wrkr/http")
local check = require("wrkr/check")
local env = require("wrkr/env")

local wfb = require("lib.wfb")

local base = wfb.base_url()

options = wfb.ramping_vus_options(wfb.max_vus(25), wfb.duration("10s"))

local FILE_15KB = "/files/15kb.bin"
local FILE_1MB = "/files/1mb.bin"

local SIZE_15KB = 15 * 1024
local SIZE_1MB = 1024 * 1024

local function header_get(headers, name)
  if type(headers) ~= "table" or type(name) ~= "string" then
    return nil
  end

  local v = headers[name]
  if v ~= nil then
    return v
  end

  local lower = string.lower(name)
  v = headers[lower]
  if v ~= nil then
    return v
  end

  -- Best-effort: some implementations might keep original case.
  for k, hv in pairs(headers) do
    if type(k) == "string" and string.lower(k) == lower then
      return hv
    end
  end

  return nil
end

local function to_int(v)
  if type(v) == "number" then
    return math.floor(v)
  end
  if type(v) ~= "string" then
    return nil
  end
  local n = tonumber(v)
  if n == nil then
    return nil
  end
  return math.floor(n)
end

local function request(method, url, opts)
  -- wrkr recently gained support for different HTTP methods. Prefer request(),
  -- but keep a minimal fallback for GET to avoid breaking if method dispatch
  -- differs between versions.
  if http.request ~= nil then
    return http.request(method, url, nil, opts)
  end
  if method == "GET" then
    return http.get(url, opts)
  end
  error("wrkr/http: http.request is required for method " .. method)
end

local headers_identity = {
  ["accept-encoding"] = "identity",
}

local function debug_res(label, res)
  if env.WFB_DEBUG ~= "1" then
    return
  end
  local cl = header_get(res.headers, "Content-Length")
  local cr = header_get(res.headers, "Content-Range")
  local bl = -1
  if type(res.body) == "string" then
    bl = #res.body
  end
  print(
    string.format(
      "[DBG] %s status=%s err=%s body_type=%s body_len=%d content_length=%s content_range=%s",
      label,
      tostring(res.status),
      tostring(res.error),
      type(res.body),
      bl,
      tostring(cl),
      tostring(cr)
    )
  )
end

local function full_get_and_validate(path, expected_size, name)
  local res = request("GET", base .. path, {
    headers = headers_identity,
    name = name,
    tags = { workload = "static_files" },
  })

  debug_res(name, res)

  local content_length = to_int(header_get(res.headers, "Content-Length"))
  local content_type = header_get(res.headers, "Content-Type")

  local ctx = {
    res = res,
    expected_size = expected_size,
    content_length = content_length,
    content_type = content_type,
  }

  check(ctx, {
    [name .. " status is 200"] = function(c)
      return c.res.status == 200
    end,
    [name .. " no transport error"] = function(c)
      return c.res.error == nil
    end,
    [name .. " has expected Content-Length"] = function(c)
      return c.content_length == c.expected_size
    end,
    [name .. " has octet-stream Content-Type"] = function(c)
      return type(c.content_type) == "string" and string.find(c.content_type, "application/octet-stream", 1, true) ~= nil
    end,
  })

  return res
end

local function verify_static_files_contract()
  -- 15kb.bin: GET twice, bytes must match.
  local r1 = full_get_and_validate(FILE_15KB, SIZE_15KB, "GET /files/15kb.bin")
  local etag_15kb = header_get(r1.headers, "ETag")
  local lm_15kb = header_get(r1.headers, "Last-Modified")
  local r2 = full_get_and_validate(FILE_15KB, SIZE_15KB, "GET /files/15kb.bin (repeat)")
  check({ etag_a = etag_15kb, lm_a = lm_15kb, etag_b = header_get(r2.headers, "ETag"), lm_b = header_get(r2.headers, "Last-Modified") }, {
    ["15kb validator stable if present"] = function(c)
      if type(c.etag_a) == "string" and c.etag_a ~= "" then
        return c.etag_a == c.etag_b
      end
      if type(c.lm_a) == "string" and c.lm_a ~= "" then
        return c.lm_a == c.lm_b
      end
      return true
    end,
  })

  -- 1mb.bin: GET twice, bytes must match.
  local r3 = full_get_and_validate(FILE_1MB, SIZE_1MB, "GET /files/1mb.bin")
  local cached_1mb_etag = header_get(r3.headers, "ETag")
  local cached_1mb_last_modified = header_get(r3.headers, "Last-Modified")

  local r4 = full_get_and_validate(FILE_1MB, SIZE_1MB, "GET /files/1mb.bin (repeat)")
  check({ etag_a = cached_1mb_etag, lm_a = cached_1mb_last_modified, etag_b = header_get(r4.headers, "ETag"), lm_b = header_get(r4.headers, "Last-Modified") }, {
    ["1mb validator stable if present"] = function(c)
      if type(c.etag_a) == "string" and c.etag_a ~= "" then
        return c.etag_a == c.etag_b
      end
      if type(c.lm_a) == "string" and c.lm_a ~= "" then
        return c.lm_a == c.lm_b
      end
      return true
    end,
  })

  -- HEAD /files/1mb.bin
  local head_res = request("HEAD", base .. FILE_1MB, {
    headers = headers_identity,
    name = "HEAD /files/1mb.bin",
    tags = { workload = "static_files" },
  })

  local head_len = to_int(header_get(head_res.headers, "Content-Length"))
  check({ res = head_res, content_length = head_len }, {
    ["HEAD status is 200"] = function(c)
      return c.res.status == 200
    end,
    ["HEAD no transport error"] = function(c)
      return c.res.error == nil
    end,
    ["HEAD has expected Content-Length"] = function(c)
      return c.content_length == SIZE_1MB
    end,
    ["HEAD body is empty"] = function(c)
      return c.res.body == nil or (type(c.res.body) == "string" and #c.res.body == 0)
    end,
  })

  -- Range request: bytes=0-1023
  local range_res = request("GET", base .. FILE_1MB, {
    headers = {
      ["accept-encoding"] = "identity",
      ["range"] = "bytes=0-1023",
    },
    name = "GET /files/1mb.bin (range 0-1023)",
    tags = { workload = "static_files" },
  })

  local content_range = header_get(range_res.headers, "Content-Range")
  local range_len = to_int(header_get(range_res.headers, "Content-Length"))
  check({ res = range_res, content_range = content_range, content_length = range_len }, {
    ["Range status is 206"] = function(c)
      return c.res.status == 206
    end,
    ["Range no transport error"] = function(c)
      return c.res.error == nil
    end,
    ["Range has correct Content-Range"] = function(c)
      return c.content_range == "bytes 0-1023/1048576"
    end,
    ["Range has Content-Length 1024"] = function(c)
      return c.content_length == 1024
    end,
  })

  -- Conditional GET: prefer ETag, else Last-Modified.
  if type(cached_1mb_etag) == "string" and cached_1mb_etag ~= "" then
    local cond = request("GET", base .. FILE_1MB, {
      headers = {
        ["accept-encoding"] = "identity",
        ["if-none-match"] = cached_1mb_etag,
      },
      name = "GET /files/1mb.bin (If-None-Match)",
      tags = { workload = "static_files" },
    })
    check(cond, {
      ["Conditional GET returns 304 (etag)"] = function(r)
        return r.status == 304
      end,
      ["Conditional GET no transport error (etag)"] = function(r)
        return r.error == nil
      end,
    })
  elseif type(cached_1mb_last_modified) == "string" and cached_1mb_last_modified ~= "" then
    local cond = request("GET", base .. FILE_1MB, {
      headers = {
        ["accept-encoding"] = "identity",
        ["if-modified-since"] = cached_1mb_last_modified,
      },
      name = "GET /files/1mb.bin (If-Modified-Since)",
      tags = { workload = "static_files" },
    })
    check(cond, {
      ["Conditional GET returns 304 (last-modified)"] = function(r)
        return r.status == 304
      end,
      ["Conditional GET no transport error (last-modified)"] = function(r)
        return r.error == nil
      end,
    })
  end
end

function Default()
  verify_static_files_contract()
end
