local countries = {"US", "DE", "FR", "JP"}
local OrderStatus = {
    UNKNOWN = 0,
    COMPLETED = 1,
    PENDING = 2,
    FAILED = 3
}
-- ~70% completed to match spec requirements
local statuses = {
    OrderStatus.COMPLETED, OrderStatus.COMPLETED, OrderStatus.COMPLETED, OrderStatus.COMPLETED, OrderStatus.COMPLETED, OrderStatus.COMPLETED, OrderStatus.COMPLETED,
    OrderStatus.PENDING, OrderStatus.FAILED, OrderStatus.PENDING
}
local categories = {"Electronics", "Books", "Clothing", "Home"}

local data_pool = {}
local pool_size = 50
local current_index = 1

-- --- Protobuf Helpers using Rust Bindings ---

local function encode_order_item(item)
    local pb = Pb.Builder()
    pb:int32(1, item.quantity)
    pb:string(2, item.category)
    pb:int64(3, item.price_cents)
    return pb:as_bytes() -- Returns encoded message bytes
end

local function encode_order(order)
    local pb = Pb.Builder()
    pb:string(1, order.id)
    pb:int32(2, order.status)
    pb:string(3, order.country)
    
    for _, item in ipairs(order.items) do
        -- Repeated field: just append multiple times
        pb:message(4, encode_order_item(item))
    end
    return pb:as_bytes()
end

local function grpc_encode_orders(orders)
    local pb = Pb.Builder()
    for _, order in ipairs(orders) do
        pb:message(1, encode_order(order))
    end
    return pb:as_grpc_frame(false)
end

local function grpc_decode_result(scanner)
    if not scanner then return {} end
    
    local result = {
        processed_orders = 0,
        amount_by_country = {},
        quantity_by_category = {},
        echoed_client_id = ""
    }
    
    while true do
        local tag_info = scanner:next()
        if not tag_info then break end
        
        local tag, wire, val_bytes = tag_info[1], tag_info[2], tag_info[3]
        
        if tag == 1 then -- processed_orders
             result.processed_orders = scanner:parse_int(val_bytes)
        elseif tag == 2 then -- amount_by_country (map)
             -- map entry is a message
             local entry_scanner = Pb.Scanner(val_bytes)
             local key
             local val
             while true do
                 local t = entry_scanner:next()
                 if not t then break end
                 if t[1] == 1 then key = entry_scanner:parse_string(t[3]) end
                 if t[1] == 2 then val = entry_scanner:parse_int(t[3]) end
             end
             if key and val then result.amount_by_country[key] = val end
             
        elseif tag == 3 then -- quantity_by_category
             local entry_scanner = Pb.Scanner(val_bytes)
             local key
             local val
             while true do
                 local t = entry_scanner:next()
                 if not t then break end
                 if t[1] == 1 then key = entry_scanner:parse_string(t[3]) end
                 if t[1] == 2 then val = entry_scanner:parse_int(t[3]) end
             end
             if key and val then result.quantity_by_category[key] = val end
             
        elseif tag == 4 then -- echoed_client_id
             result.echoed_client_id = scanner:parse_string(val_bytes)
        end
    end
    
    return result
end

-- --- End Protobuf Helpers ---

local function generate_entry()
    local num_orders = 100
    
    local orders = {}
    local expected_processed = 0
    local expected_results = {}
    local expected_category_stats = {}

    local client_id = uuid_v4() -- Using native Rust UUID

    -- Initialize maps
    for _, c in ipairs(countries) do expected_results[c] = 0 end
    for _, c in ipairs(categories) do expected_category_stats[c] = 0 end

    for i = 0, num_orders - 1 do
        local status = statuses[(i % #statuses) + 1]
        local country = countries[(i % #countries) + 1]
        
        local items = {}
        local order_amount = 0
        
        for j = 0, 2 do
            local price = math.random(1000, 10000)
            local quantity = math.random(1, 5)
            local category = categories[((i + j) % #categories) + 1]
            
            order_amount = order_amount + (price * quantity)
            table.insert(items, {
                quantity = quantity,
                price_cents = price,
                category = category
            })
            
            if status == OrderStatus.COMPLETED then
                expected_category_stats[category] = expected_category_stats[category] + quantity
            end
        end
        
        table.insert(orders, {
            id = tostring(i + 1),
            status = status,
            country = country,
            items = items
        })
        
        if status == OrderStatus.COMPLETED then
            expected_processed = expected_processed + 1
            expected_results[country] = expected_results[country] + order_amount
        end
    end

    local encoded = grpc_encode_orders(orders)

    return {
        encoded_orders = encoded,
        client_id = client_id,
        expected_processed = expected_processed,
        expected_results = expected_results,
        expected_category_stats = expected_category_stats
    }
end

setup = function(ctx)
    math.randomseed(os.time() + ctx:vu())
    for i = 1, pool_size do
        data_pool[i] = generate_entry()
    end
end

scenario = function(ctx)
    local data = data_pool[current_index]
    current_index = (current_index % pool_size) + 1

    local headers = {
        ["content-type"] = "application/grpc",
        ["te"] = "trailers",
        ["grpc-timeout"] = "10S", -- 10 Seconds
        ["x-client-id"] = data.client_id
    }
    
    local resp = ctx:http({
        method = "POST",
        url = "/AnalyticsService/AggregateOrders",
        headers = headers,
        body = data.encoded_orders
    })
    
    ctx:assert(resp:status() == 200, "Status is not 200: " .. resp:status())
    
    local grpc_status = resp:header("grpc-status")
    if grpc_status and grpc_status ~= "0" then
        local msg = resp:header("grpc-message") or "unknown"
        ctx:assert(false, "gRPC Error: Status=" .. grpc_status .. " Msg=" .. msg)
    end
    
    local scanner = resp:grpc_scanner()
    if not scanner then
         local body_len = 0
         -- We can try to access resp.bytes length if we expose it, or just assume it is issue
         -- The Response object has .bytes field
         local b = resp.bytes
         if b then body_len = #b end
         ctx:assert(false, "gRPC response body invalid or empty. Len: " .. body_len)
    end
    
    local result = grpc_decode_result(scanner)
    
    ctx:assert(result.echoed_client_id == data.client_id, "Client ID mismatch. Expected: " .. data.client_id .. ", Got: '" .. (result.echoed_client_id or "nil") .. "'")
    
    ctx:assert(result.processed_orders == data.expected_processed, 
        string.format("processed_orders mismatch: expected %d, got %d", data.expected_processed, result.processed_orders))
        
    for k, v in pairs(data.expected_results) do
        local actual = result.amount_by_country[k] or 0
        ctx:assert(actual == v, string.format("amount mismatch for %s: expected %d, got %d", k, v, actual))
    end
    
     for k, v in pairs(data.expected_category_stats) do
        local actual = result.quantity_by_category[k] or 0
        ctx:assert(actual == v, string.format("quantity mismatch for %s: expected %d, got %d", k, v, actual))
    end
end
