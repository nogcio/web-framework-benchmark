local counter = 0

setup = function(ctx)
    math.randomseed(os.time() + ctx:vu())
    ctx:track_status_codes(false)
end

local function validate_post(ctx, post, check_views_sort, last_val)
    ctx:assert(type(post) == "table", "post must be a table")
    ctx:assert(type(post.title) == "string", "post.title must be a string")
    ctx:assert(type(post.content) == "string", "post.content must be a string")
    ctx:assert(type(post.views) == "number", "post.views must be a number")
    ctx:assert(type(post.createdAt) == "string", "post.createdAt must be a string")
    
    if type(post.id) ~= "number" and type(post.id) ~= "string" then
       ctx:assert(false, "post.id must be a number or string")
    end
    
    if check_views_sort and last_val then
        ctx:assert(post.views <= last_val, "trending posts not sorted by views: " .. post.views .. " > " .. last_val)
    end
    return post.views
end

scenario = function(ctx)
    counter = counter + 1
    local r = counter % 100
    
    -- 1. Happy Path (95%)
    if r < 95 then
        local id = math.random(1, 10000)
        local email = "user_" .. id .. "@example.com"
        local url = "/db/user-profile/" .. email
        
        local res = ctx:get(url)
        
        ctx:assert(res:status() == 200, "unexpected status: " .. res:status())
        
        local ct = res:header("content-type")
        ctx:assert(ct ~= nil and string.find(ct, "application/json") ~= nil, "invalid content-type: " .. (ct or "nil"))
        
        local body = res:json()
        
        -- User validation
        ctx:assert(body.username == "user_" .. id, "username mismatch")
        ctx:assert(body.email == email, "email mismatch")
        ctx:assert(type(body.createdAt) == "string", "createdAt must be string")
        ctx:assert(type(body.lastLogin) == "string", "lastLogin must be string (and updated from NULL)")
        
        -- Settings validation
        ctx:assert(type(body.settings) == "table", "settings must be object")
        ctx:assert(body.settings.theme == "dark", "settings.theme mismatch")
        ctx:assert(body.settings.notifications == true, "settings.notifications mismatch")
        ctx:assert(body.settings.language == "en", "settings.language mismatch")
        
        -- Posts validation
        ctx:assert(type(body.posts) == "table", "posts must be array")
        ctx:assert(#body.posts == 10, "posts length mismatch: " .. #body.posts)
        for _, post in ipairs(body.posts) do
            validate_post(ctx, post)
        end
        
        -- Trending validation
        ctx:assert(type(body.trending) == "table", "trending must be array")
        ctx:assert(#body.trending == 5, "trending length mismatch: " .. #body.trending)
        local last_views = nil
        for _, post in ipairs(body.trending) do
            last_views = validate_post(ctx, post, true, last_views)
        end

    -- 2. Negative Case: User Not Found (5%)
    else
        local email = "user_999999@example.com"
        local url = "/db/user-profile/" .. email
        local res = ctx:get(url)
        ctx:assert(res:status() == 404, "expected 404 for non-existent user, got " .. res:status())
    end
end
