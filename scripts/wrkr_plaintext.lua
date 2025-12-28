function scenario(ctx)
  local res = ctx:get("/plaintext")

  ctx:assert(res.status == 200, "unexpected status: " .. res.status)
  
  -- Zero-copy body check
  ctx:assert(res:check_body("Hello, World!"), "unexpected body")
  
  -- Efficient header check without creating a table
  local ct = res:header("content-type")
  ctx:assert(ct ~= nil and string.find(ct, "text/plain") ~= nil, "invalid content-type: " .. (ct or "nil"))
end
