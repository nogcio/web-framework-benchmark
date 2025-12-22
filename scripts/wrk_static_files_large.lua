-- request one of the files repeatedly
local files = {"/files/10mb.bin"}

function init(args)
  math.randomseed(os.time() + math.floor(os.clock() * 100000))
end

function request()
  local idx = math.random(#files)
  return wrk.format(nil, files[idx])
end
