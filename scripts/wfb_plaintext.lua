local http = require("wrkr/http")
local check = require("wrkr/check")

local wfb = require("lib.wfb")

local base = wfb.base_url()

Options = wfb.ramping_vus_options(wfb.max_vus(100), wfb.duration("10s"))

function Default()
  local res = http.get(base .. "/plaintext", {
    name = "GET /plaintext",
    tags = { workload = "plaintext" },
  })

  check(res, {
    ["status is 200"] = function(r)
      return r.status == 200
    end,
    ["no transport error"] = function(r)
      return r.error == nil
    end,
    ["body is expected"] = function(r)
      return r.body == "Hello, World!"
    end,
  })
end
