local http = require("wrkr/http")
local check = require("wrkr/check")
local json = require("wrkr/json")
local vu = require("wrkr/vu")

local Pool = require("lib.pool")
local wfb = require("lib.wfb")

local base = wfb.base_url()

options = wfb.ramping_vus_options(wfb.max_vus(50), wfb.duration("10s"))

local countries = { "US", "DE", "FR", "UK", "JP" }
local statuses = { "completed", "pending", "failed" }
local categories = { "Electronics", "Books", "Clothing", "Home" }

local function init_zero_map(keys)
  local out = {}
  for _, k in ipairs(keys) do
    out[k] = 0
  end
  return out
end

local function generate_case()
  local num_orders = 100

  local orders = {}
  local expected_processed = 0
  local expected_results = init_zero_map(countries)
  local expected_category_stats = init_zero_map(categories)

  for i = 0, num_orders - 1 do
    local status = statuses[(i % #statuses) + 1]
    local country = countries[(i % #countries) + 1]

    local items = {}
    local total_amount = 0

    for j = 0, 2 do
      local price = math.random(1000, 10000)
      local quantity = math.random(1, 5)
      local category = categories[((i + j) % #categories) + 1]

      total_amount = total_amount + (price * quantity)
      table.insert(items, {
        quantity = quantity,
        price = price,
        category = category,
      })

      if status == "completed" then
        expected_category_stats[category] = expected_category_stats[category] + quantity
      end
    end

    table.insert(orders, {
      id = tostring(i + 1),
      status = status,
      amount = total_amount,
      country = country,
      items = items,
    })

    if status == "completed" then
      expected_processed = expected_processed + 1
      expected_results[country] = expected_results[country] + total_amount
    end
  end

  return {
    orders = orders,
    expected_processed = expected_processed,
    expected_results = expected_results,
    expected_category_stats = expected_category_stats,
  }
end

local pool = Pool.new({
  size = 200,
  generate = generate_case,
})

function Default()
  pool:ensure_initialized(vu.id())

  local data = pool:next()
  local res = http.post(base .. "/json/aggregate", data.orders, {
    headers = {
      accept = "application/json",
    },
    name = "POST /json/aggregate",
    tags = { workload = "json_aggregate" },
  })

  local decode_ok, body = pcall(json.decode, res.body)

  local ctx = {
    res = res,
    decode_ok = decode_ok,
    body = body,
    expected = data,
  }

  check(ctx, {
    ["status is 200"] = function(c)
      return c.res.status == 200
    end,
    ["no transport error"] = function(c)
      return c.res.error == nil
    end,
    ["body is valid json"] = function(c)
      return c.decode_ok == true
    end,
    ["body is json object"] = function(c)
      return c.decode_ok == true and type(c.body) == "table"
    end,
    ["processedOrders matches"] = function(c)
      if c.decode_ok ~= true or type(c.body) ~= "table" then
        return false
      end
      return c.body.processedOrders == c.expected.expected_processed
    end,
    ["results match"] = function(c)
      if c.decode_ok ~= true or type(c.body) ~= "table" then
        return false
      end
      return wfb.totals_match(c.body.results, c.expected.expected_results)
    end,
    ["categoryStats match"] = function(c)
      if c.decode_ok ~= true or type(c.body) ~= "table" then
        return false
      end
      return wfb.totals_match(c.body.categoryStats, c.expected.expected_category_stats)
    end,
  })
end
