wrk.method = "GET"
wrk.headers["Connection"] = "keep-alive"

-- request one of the files repeatedly
local files = {"/files/15kb.bin", "/files/1mb.bin", "/files/10mb.bin"}

request = function()
    local idx = math.random(#files)
    return wrk.format(nil, files[idx])
end
