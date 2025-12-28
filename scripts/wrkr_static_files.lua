local files = {
    { name = "15kb.bin", size = 15 * 1024 },
    { name = "1mb.bin", size = 1024 * 1024 },
    { name = "10mb.bin", size = 10 * 1024 * 1024 }
}

setup = function(ctx)
    -- No specific setup needed
end

scenario = function(ctx)
    -- 1) Full GET for all files
    local file_responses = {}
    for _, file in ipairs(files) do
        local url = "/files/" .. file.name
        local resp = ctx:http({
            method = "GET",
            url = url,
            headers = { ["Accept-Encoding"] = "identity" }
        })

        ctx:assert(resp:status() == 200, string.format("GET %s: Status is not 200: %d", file.name, resp:status()))
        
        local cl = tonumber(resp:header("content-length"))
        ctx:assert(cl == file.size, string.format("GET %s: Content-Length mismatch: expected %d, got %s", file.name, file.size, tostring(cl)))
        
        local ct = resp:header("content-type") or ""
        ctx:assert(string.find(ct, "application/octet-stream"), string.format("GET %s: Content-Type mismatch: expected application/octet-stream, got %s", file.name, ct))
        
        file_responses[file.name] = resp

        -- Verify stability (2nd request)
        local resp2 = ctx:http({
            method = "GET",
            url = url,
            headers = { ["Accept-Encoding"] = "identity" }
        })
        ctx:assert(resp2:status() == 200, string.format("GET %s (2nd): Status is not 200: %d", file.name, resp2:status()))

        ctx:assert(resp2:check_body_resp(resp), string.format("GET %s (2nd): Body mismatch", file.name))
    end

    -- 2) HEAD for 1mb.bin
    local file_1mb = files[2]
    local url_1mb = "/files/" .. file_1mb.name
    
    local resp_head = ctx:http({
        method = "HEAD",
        url = url_1mb,
        headers = { ["Accept-Encoding"] = "identity" }
    })
    
    ctx:assert(resp_head:status() == 200, "HEAD 1mb.bin: Status is not 200: " .. resp_head:status())
    local cl_head = tonumber(resp_head:header("content-length"))
    ctx:assert(cl_head == file_1mb.size, string.format("HEAD 1mb.bin: Content-Length mismatch: expected %d, got %s", file_1mb.size, tostring(cl_head)))
    
    -- HEAD body should be empty. We can use check_body with empty string.
    ctx:assert(resp_head:check_body(""), "HEAD 1mb.bin: Body is not empty")

    -- 3) Range Request for 1mb.bin (first 1024 bytes)
    local range_end = 1023
    local resp_range = ctx:http({
        method = "GET",
        url = url_1mb,
        headers = { 
            ["Accept-Encoding"] = "identity",
            ["Range"] = "bytes=0-" .. range_end
        }
    })
    
    ctx:assert(resp_range:status() == 206, "Range 1mb.bin: Status is not 206: " .. resp_range:status())
    
    local cr = resp_range:header("content-range") or ""
    local expected_cr = string.format("bytes 0-%d/%d", range_end, file_1mb.size)
    ctx:assert(cr == expected_cr, string.format("Range 1mb.bin: Content-Range mismatch: expected '%s', got '%s'", expected_cr, cr))
    
    -- Verify range content matches prefix of full file (efficiently)
    local full_resp_1mb = file_responses["1mb.bin"]
    ctx:assert(resp_range:check_body_resp_prefix(full_resp_1mb, 1024), "Range 1mb.bin: Body content mismatch (does not match prefix of full file)")

    -- 4) Conditional GET (Cache Validation) for 1mb.bin
    -- We need the headers from a full GET first
    local resp_full = file_responses["1mb.bin"]
    
    local etag = resp_full:header("etag")
    local last_modified = resp_full:header("last-modified")
    
    if etag then
        local resp_304 = ctx:http({
            method = "GET",
            url = url_1mb,
            headers = { ["If-None-Match"] = etag }
        })
        ctx:assert(resp_304:status() == 304, string.format("Conditional GET (ETag): Status is not 304: %d", resp_304:status()))
    end
    
    if last_modified then
        local resp_304 = ctx:http({
            method = "GET",
            url = url_1mb,
            headers = { ["If-Modified-Since"] = last_modified }
        })
        ctx:assert(resp_304:status() == 304, string.format("Conditional GET (Last-Modified): Status is not 304: %d", resp_304:status()))
    end
end
