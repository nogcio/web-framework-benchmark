local countries = {"US", "DE", "FR", "UK", "JP"}
local statuses = {"completed", "pending", "failed"}
local categories = {"Electronics", "Books", "Clothing", "Home"}

setup = function(ctx)
    -- Seed random number generator with time and VU ID to ensure uniqueness per VU
    math.randomseed(os.time() + ctx:vu())
end

scenario = function(ctx)
    local num_orders = 500
    
    local orders = {}
    local expected_processed = 0
    local expected_results = {}
    local expected_category_stats = {}

    -- Initialize maps
    for _, c in ipairs(countries) do expected_results[c] = 0 end
    for _, c in ipairs(categories) do expected_category_stats[c] = 0 end

    for i = 0, num_orders - 1 do
        local status = statuses[(i % #statuses) + 1]
        local country = countries[(i % #countries) + 1]
        
        local items = {}
        local total_amount = 0
        
        for j = 0, 2 do
            -- price: 1000 to 10000 (10.00 to 99.99)
            local price = math.random(1000, 10000)
            -- quantity: 1 to 5
            local quantity = math.random(1, 5)
            local category = categories[((i + j) % #categories) + 1]
            
            total_amount = total_amount + (price * quantity)
            table.insert(items, {
                quantity = quantity,
                price = price,
                category = category
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
            items = items
        })
        
        if status == "completed" then
            expected_processed = expected_processed + 1
            expected_results[country] = expected_results[country] + total_amount
        end
    end
    
    local resp = ctx:http({
        method = "POST",
        url = "/json/aggregate",
        headers = { ["Content-Type"] = "application/json" },
        body = orders
    })
    
    ctx:assert(resp:status() == 200, "Status is not 200: " .. resp:status())
    
    local body = resp:json()
    
    ctx:assert(type(body) == "table", "Response body is not a JSON object")
    
    ctx:assert(body.processedOrders == expected_processed, 
        string.format("processedOrders mismatch: expected %d, got %s", expected_processed, tostring(body.processedOrders)))
    
    ctx:assert(type(body.results) == "table", "body.results is not a table")
    for k, v in pairs(expected_results) do
        local actual = body.results[k] or 0
        ctx:assert(actual == v, string.format("results mismatch for %s: expected %d, got %d", k, v, actual))
    end
    
    ctx:assert(type(body.categoryStats) == "table", "body.categoryStats is not a table")
    for k, v in pairs(expected_category_stats) do
        local actual = body.categoryStats[k] or 0
        ctx:assert(actual == v, string.format("categoryStats mismatch for %s: expected %d, got %d", k, v, actual))
    end
end
