local counter = 1
local threads = {}

function setup(thread)
   thread:set("id", counter)
   table.insert(threads, thread)
   counter = counter + 1
end

function init(args)
   requests  = 0
   responses = 0
   errors    = 0
end

function request()
   requests = requests + 1
   return wrk.request()
end

function response(status, headers, body)
   responses = responses + 1
   if status ~= 200 then
      errors = errors + 1
      return
   end
   if body ~= "Hello, World!" then
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