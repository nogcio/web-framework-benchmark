-- scripts/wrk_tweet_service.lua

local counter = 1
local threads = {}

function setup(thread)
    thread:set("id", counter)
    table.insert(threads, thread)
    counter = counter + 1
end

function init(args)
    -- Better seeding
    math.randomseed(id + os.time() + math.floor(os.clock() * 1000000))
    
    -- User pool: list of { username=..., token=... }
    users = {}
    
    -- Queue for users who have registered but need to login
    login_queue = {}
    
    -- Track pending requests to handle callbacks
    -- Key: req_id, Value: { type="register"|"login"|"action", data=... }
    pending_requests = {}
    
    -- Cache of known tweet IDs
    known_tweets = {}
    
    -- Request headers (built per-request in request())
    headers = {}
    
    seq = 0
    errors = 0

    -- Diagnostics
    resp_total = 0
    resp_2xx = 0
    resp_3xx = 0
    resp_4xx = 0
    resp_5xx = 0
    resp_other = 0

    missing_reqid = 0
    missing_ctx = 0

    register_ok = 0
    register_fail = 0
    login_ok = 0
    login_fail = 0
    action_ok = 0
    action_fail = 0
end

function request()
    seq = seq + 1
    local reqid = string.format("%d-%d", id, seq)
    local hdrs = { ["x-request-id"] = reqid }
    
    -- Priority 1: Complete Logins for registered users
    if #login_queue > 0 then
        local u = table.remove(login_queue, 1)
        pending_requests[reqid] = { type = "login", username = u.username }
        hdrs["Content-Type"] = "application/json"
        local body = string.format('{"username": "%s", "password": "%s"}', u.username, u.password)
        return wrk.format("POST", "/api/auth/login", hdrs, body)
    end
    
    -- Priority 2: Register new users (10% chance or if pool is empty)
    -- We try to take a user from the pool first
    local user = nil
    if #users > 0 and math.random() > 0.10 then
        user = table.remove(users)
    end

    -- If no user available (pool empty or 10% chance hit), register new one
    if not user then
        local username = "user_" .. id .. "_" .. os.time() .. "_" .. seq
        local password = "password123"
        pending_requests[reqid] = { type = "register", username = username, password = password }
        hdrs["Content-Type"] = "application/json"
        local body = string.format('{"username": "%s", "password": "%s"}', username, password)
        return wrk.format("POST", "/api/auth/register", hdrs, body)
    end
    
    -- Priority 3: Benchmark Actions with existing users
    hdrs["Authorization"] = "Bearer " .. user.token
    pending_requests[reqid] = { type = "action", user = user }
    
    local r = math.random(1, 100)
    
    -- 60% Feed
    if r <= 60 then
        pending_requests[reqid].action = "feed"
        return wrk.format("GET", "/api/feed", hdrs)
    end

    -- 20% Get Tweet
    if r <= 80 then
        if #known_tweets > 0 then
            local tid = known_tweets[math.random(1, #known_tweets)]
            pending_requests[reqid].action = "get_tweet"
            return wrk.format("GET", "/api/tweets/" .. tid, hdrs)
        else
            pending_requests[reqid].action = "feed_fallback"
            return wrk.format("GET", "/api/feed", hdrs)
        end
    end

    -- 10% Post Tweet
    if r <= 90 then
        local content = "Tweet content " .. os.time() .. " " .. seq
        local body = string.format('{"content": "%s"}', content)
        pending_requests[reqid].action = "post_tweet"
        hdrs["Content-Type"] = "application/json"
        return wrk.format("POST", "/api/tweets", hdrs, body)
    end

    -- 10% Like Tweet
    if r <= 100 then
        if #known_tweets > 0 then
            local tid = known_tweets[math.random(1, #known_tweets)]
            pending_requests[reqid].action = "like"
            -- IMPORTANT: no Content-Type for empty-body POST (Fastify may treat empty JSON body as invalid)
            return wrk.format("POST", "/api/tweets/" .. tid .. "/like", hdrs)
        else
            pending_requests[reqid].action = "feed_fallback"
            return wrk.format("GET", "/api/feed", hdrs)
        end
    end
end

function response(status, headers, body)
    resp_total = resp_total + 1

    if status >= 200 and status < 300 then
        resp_2xx = resp_2xx + 1
    elseif status >= 300 and status < 400 then
        resp_3xx = resp_3xx + 1
    elseif status >= 400 and status < 500 then
        resp_4xx = resp_4xx + 1
    elseif status >= 500 and status < 600 then
        resp_5xx = resp_5xx + 1
    else
        resp_other = resp_other + 1
    end

    local reqid = headers["x-request-id"] or headers["X-Request-Id"] or headers["X-Request-ID"]
    if not reqid then
        missing_reqid = missing_reqid + 1
        errors = errors + 1
        return
    end

    local ctx = pending_requests[reqid]
    
    if ctx then
        pending_requests[reqid] = nil -- Cleanup
        
        if ctx.type == "register" then
            if status == 201 or status == 200 then
                -- Registration successful, queue for login
                table.insert(login_queue, { username = ctx.username, password = ctx.password })
                register_ok = register_ok + 1
            else
                register_fail = register_fail + 1
            end
        elseif ctx.type == "login" then
            if status == 200 then
                local t = string.match(body, '"token"%s*:%s*"(.-)"')
                if t then
                    table.insert(users, { username = ctx.username, token = t })
                    login_ok = login_ok + 1
                else
                    login_fail = login_fail + 1
                    errors = errors + 1
                end
            else
                login_fail = login_fail + 1
            end
        elseif ctx.type == "action" then
            -- Return user to pool
            if ctx.user then
                table.insert(users, ctx.user)
            end

            if status >= 200 and status < 300 then
                action_ok = action_ok + 1
            else
                action_fail = action_fail + 1
            end

            -- Per-action failure breakdown (keep it simple)
            if ctx.action ~= nil and status >= 400 and status < 600 then
                local key = "action_fail_" .. ctx.action
                _G[key] = (_G[key] or 0) + 1
            end

            -- Harvest Tweet IDs
            if body and (status == 200 or status == 201) then
                for tid in string.gmatch(body, '"id"%s*:%s*(%d+)') do
                    table.insert(known_tweets, tid)
                end
                -- Cap cache
                if #known_tweets > 500 then
                    local idx = math.random(1, 500)
                    known_tweets[idx] = known_tweets[#known_tweets]
                    table.remove(known_tweets)
                end
            end
        end
    else
        missing_ctx = missing_ctx + 1
    end
end

function done(summary, latency, requests)
  local total_errors = 0
    local total_resp = 0
    local total_2xx = 0
    local total_3xx = 0
    local total_4xx = 0
    local total_5xx = 0
    local total_other = 0
    local total_missing_reqid = 0
    local total_missing_ctx = 0

    local total_register_ok = 0
    local total_register_fail = 0
    local total_login_ok = 0
    local total_login_fail = 0
    local total_action_ok = 0
    local total_action_fail = 0

  for _, thread in ipairs(threads) do
    local thread_errors = thread:get("errors")
    total_errors = total_errors + thread_errors

        total_resp = total_resp + (thread:get("resp_total") or 0)
        total_2xx = total_2xx + (thread:get("resp_2xx") or 0)
        total_3xx = total_3xx + (thread:get("resp_3xx") or 0)
        total_4xx = total_4xx + (thread:get("resp_4xx") or 0)
        total_5xx = total_5xx + (thread:get("resp_5xx") or 0)
        total_other = total_other + (thread:get("resp_other") or 0)
        total_missing_reqid = total_missing_reqid + (thread:get("missing_reqid") or 0)
        total_missing_ctx = total_missing_ctx + (thread:get("missing_ctx") or 0)

        total_register_ok = total_register_ok + (thread:get("register_ok") or 0)
        total_register_fail = total_register_fail + (thread:get("register_fail") or 0)
        total_login_ok = total_login_ok + (thread:get("login_ok") or 0)
        total_login_fail = total_login_fail + (thread:get("login_fail") or 0)
        total_action_ok = total_action_ok + (thread:get("action_ok") or 0)
        total_action_fail = total_action_fail + (thread:get("action_fail") or 0)
  end
    -- IMPORTANT: don't print "Errors:" here; the runner already counts
    -- wrk's built-in "Non-2xx or 3xx responses" line. Printing "Errors:"
    -- would double-count in src/wrk.rs.
    print("LuaErrors: " .. total_errors)
    print(string.format(
        "Responses: total=%d 2xx=%d 3xx=%d 4xx=%d 5xx=%d other=%d",
        total_resp, total_2xx, total_3xx, total_4xx, total_5xx, total_other
    ))
    print(string.format(
        "Correlation: missing_reqid=%d missing_ctx=%d",
        total_missing_reqid, total_missing_ctx
    ))
    print(string.format(
        "Flow: register ok=%d fail=%d | login ok=%d fail=%d | action ok=%d fail=%d",
        total_register_ok, total_register_fail,
        total_login_ok, total_login_fail,
        total_action_ok, total_action_fail
    ))

    -- Best-effort action breakdown (aggregated via globals)
    local keys = {"feed","feed_fallback","get_tweet","post_tweet","like"}
    local parts = {}
    for _, k in ipairs(keys) do
        local c = 0
        for _, thread in ipairs(threads) do
            c = c + (thread:get("action_fail_" .. k) or 0)
        end
        table.insert(parts, k .. "=" .. c)
    end
    print("ActionFail: " .. table.concat(parts, " "))
end