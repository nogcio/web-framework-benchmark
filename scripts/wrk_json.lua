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
  -- embedded example.json
  body_data = [[
{
  "web-app": {
    "servlet": [
      {
        "servlet-name": "cofaxCDS",
        "servlet-class": "org.cofax.cds.CDSServlet",
        "init-param": {
          "configGlossary:installationAt": "Philadelphia, PA",
          "configGlossary:adminEmail": "ksm@pobox.com",
          "configGlossary:poweredBy": "Cofax",
          "configGlossary:poweredByIcon": "/images/cofax.gif",
          "configGlossary:staticPath": "/content/static",
          "templateProcessorClass": "org.cofax.WysiwygTemplate",
          "templateLoaderClass": "org.cofax.FilesTemplateLoader",
          "templatePath": "templates",
          "templateOverridePath": "",
          "defaultListTemplate": "listTemplate.htm",
          "defaultFileTemplate": "articleTemplate.htm",
          "useJSP": false,
          "jspListTemplate": "listTemplate.jsp",
          "jspFileTemplate": "articleTemplate.jsp",
          "cachePackageTagsTrack": 200,
          "cachePackageTagsStore": 200,
          "cachePackageTagsRefresh": 60,
          "cacheTemplatesTrack": 100,
          "cacheTemplatesStore": 50,
          "cacheTemplatesRefresh": 15,
          "cachePagesTrack": 200,
          "cachePagesStore": 100,
          "cachePagesRefresh": 10,
          "cachePagesDirtyRead": 10,
          "searchEngineListTemplate": "forSearchEnginesList.htm",
          "searchEngineFileTemplate": "forSearchEngines.htm",
          "searchEngineRobotsDb": "WEB-INF/robots.db",
          "useDataStore": true,
          "dataStoreClass": "org.cofax.SqlDataStore",
          "redirectionClass": "org.cofax.SqlRedirection",
          "dataStoreName": "cofax",
          "dataStoreDriver": "com.microsoft.jdbc.sqlserver.SQLServerDriver",
          "dataStoreUrl": "jdbc:microsoft:sqlserver://LOCALHOST:1433;DatabaseName=goon",
          "dataStoreUser": "sa",
          "dataStorePassword": "dataStoreTestQuery",
          "dataStoreTestQuery": "SET NOCOUNT ON;select test='test';",
          "dataStoreLogFile": "/usr/local/tomcat/logs/datastore.log",
          "dataStoreInitConns": 10,
          "dataStoreMaxConns": 100,
          "dataStoreConnUsageLimit": 100,
          "dataStoreLogLevel": "debug",
          "maxUrlLength": 500
        }
      },
      {
        "servlet-name": "cofaxEmail",
        "servlet-class": "org.cofax.cds.EmailServlet",
        "init-param": {
          "mailHost": "mail1",
          "mailHostOverride": "mail2"
        }
      },
      {
        "servlet-name": "cofaxAdmin",
        "servlet-class": "org.cofax.cds.AdminServlet"
      },
      {
        "servlet-name": "fileServlet",
        "servlet-class": "org.cofax.cds.FileServlet"
      },
      {
        "servlet-name": "cofaxTools",
        "servlet-class": "org.cofax.cms.CofaxToolsServlet",
        "init-param": {
          "templatePath": "toolstemplates/",
          "log": 1,
          "logLocation": "/usr/local/tomcat/logs/CofaxTools.log",
          "logMaxSize": "",
          "dataLog": 1,
          "dataLogLocation": "/usr/local/tomcat/logs/dataLog.log",
          "dataLogMaxSize": "",
          "removePageCache": "/content/admin/remove?cache=pages&id=",
          "removeTemplateCache": "/content/admin/remove?cache=templates&id=",
          "fileTransferFolder": "/usr/local/tomcat/webapps/content/fileTransferFolder",
          "lookInContext": 1,
          "adminGroupID": 4,
          "betaServer": true
        }
      }
    ],
    "servlet-mapping": {
      "cofaxCDS": "/",
      "cofaxEmail": "/cofaxutil/aemail/*",
      "cofaxAdmin": "/admin/*",
      "fileServlet": "/static/*",
      "cofaxTools": "/tools/*"
    },
    "taglib": {
      "taglib-uri": "cofax.tld",
      "taglib-location": "/WEB-INF/tlds/cofax.tld"
    }
  }
}
]]
    -- extract servlet-name values into a list for random selection
    servlet_names = {}
    for name in body_data:gmatch('"servlet%-name"%s*:%s*"([^"]+)"') do
      table.insert(servlet_names, name)
    end
    if #servlet_names == 0 then
      servlet_names = { "cofaxCDS" }
    end
end

function request()
  seq = seq + 1
  local thread_id = 0
  if wrk.thread and wrk.thread.id then
    thread_id = tonumber(wrk.thread.id) or 0
  end
  local reqid = string.format("%d-%d", thread_id, seq)
  -- choose replacement: pick random existing servlet-name
  local from = servlet_names[math.random(1, #servlet_names)]
  local to = string.format("REPLACED-%d-%d", thread_id, seq)
  pending[reqid] = { to = to }
  local path = string.format("/json/%s/%s", from, to)
  local hdrs = { ["x-request-id"] = reqid, ["Content-Type"] = "application/json" }
  local b = body_data or "{}"
  return wrk.format("POST", path, hdrs, b)
end

function response(status, headers, body)
  if status ~= 200 then
    errors = errors + 1
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
  else
    errors = errors + 1
    return
  end
  if not expected then
    errors = errors + 1
    return
  end
  if not body:find(expected.to, 1, true) then
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