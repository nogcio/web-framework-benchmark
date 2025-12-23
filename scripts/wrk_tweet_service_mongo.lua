-- scripts/wrk_tweet_service_mongo.lua

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
    
    -- Request headers
    headers = {}
    headers["Content-Type"] = "application/json"
    
    seq = 0
    errors = 0
end

function request()
    seq = seq + 1
    local reqid = string.format("%d-%d", id, seq)
    headers["x-request-id"] = reqid
    
    -- Priority 1: Complete Logins for registered users
    if #login_queue > 0 then
        local u = table.remove(login_queue, 1)
        pending_requests[reqid] = { type = "login", username = u.username }
        headers["Authorization"] = nil
        local body = string.format('{"username": "%s", "password": "%s"}', u.username, u.password)
        return wrk.format("POST", "/api/auth/login", headers, body)
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
        headers["Authorization"] = nil
        local body = string.format('{"username": "%s", "password": "%s"}', username, password)
        return wrk.format("POST", "/api/auth/register", headers, body)
    end
    
    -- Priority 3: Benchmark Actions with existing users
    headers["Authorization"] = "Bearer " .. user.token
    pending_requests[reqid] = { type = "action", user = user }
    
    local r = math.random(1, 100)
    
    -- 60% Feed
    if r <= 60 then
        return wrk.format("GET", "/api/feed", headers)
    end

    -- 20% Get Tweet
    if r <= 80 then
        if #known_tweets > 0 then
            local tid = known_tweets[math.random(1, #known_tweets)]
            return wrk.format("GET", "/api/tweets/" .. tid, headers)
        else
            return wrk.format("GET", "/api/feed", headers)
        end
    end

    -- 10% Post Tweet
    if r <= 90 then
        local content = "Tweet content " .. os.time() .. " " .. seq
        local body = string.format('{"content": "%s"}', content)
        return wrk.format("POST", "/api/tweets", headers, body)
    end

    -- 10% Like Tweet
    if r <= 100 then
        if #known_tweets > 0 then
            local tid = known_tweets[math.random(1, #known_tweets)]
            return wrk.format("POST", "/api/tweets/" .. tid .. "/like", headers)
        else
            return wrk.format("GET", "/api/feed", headers)
        end
    end
end

function response(status, headers, body)
    local reqid = headers["x-request-id"]
    local ctx = pending_requests[reqid]
    
    if ctx then
        pending_requests[reqid] = nil -- Cleanup
        
        if ctx.type == "register" then
            if status == 201 or status == 200 then
                -- Registration successful, queue for login
                table.insert(login_queue, { username = ctx.username, password = ctx.password })
            end
        elseif ctx.type == "login" then
            if status == 200 then
                local t = string.match(body, '"token"%s*:%s*"(.-)"')
                if t then
                    table.insert(users, { username = ctx.username, token = t })
                end
            else
                errors = errors + 1
            end
        elseif ctx.type == "action" then
            -- Return user to pool
            if ctx.user then
                table.insert(users, ctx.user)
            end

            -- Harvest Tweet IDs
            if body and (status == 200 or status == 201) then
                -- Match quoted string IDs for MongoDB
                for tid in string.gmatch(body, '"id"%s*:%s*"([^"]+)"') do
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
