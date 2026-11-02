local M = {}
function M.basic(f) local ok, err = pcall(f) return ok, err end
function M.withargs(f, a, b) local ok, r = pcall(f, a, b) if not ok then return "fail" end return r end
function M.inline() local ok, e = pcall(function() error("x", 0) end) return ok, e end
function M.tableerr() local ok, e = pcall(error, {code = 1}) return ok, e.code end
function M.handler(f) return xpcall(f, function(m) return "h:" .. tostring(m) end) end
function M.assertion(a) assert(a, "need a") return a end
function M.assertcall(f) return assert(f()) end
function M.nested()
    local ok, r = pcall(function()
        local ok2, r2 = pcall(function() error("in", 0) end)
        if not ok2 then error(r2 .. "out", 0) end
        return 1
    end)
    return ok, r
end
function M.retry(f, n)
    for i = 1, n do
        local ok, v = pcall(f, i)
        if ok then return v, i end
    end
    return nil, n
end
function M.wrap() local g = coroutine.wrap(function(a) local b = coroutine.yield(a + 1) return b * 2 end) return g(1), g(2) end
function M.create()
    local c = coroutine.create(function() for i = 1, 2 do coroutine.yield(i) end end)
    local out = {}
    while true do
        local ok, v = coroutine.resume(c)
        if not ok or v == nil then break end
        table.insert(out, v)
    end
    return #out, coroutine.status(c)
end
function M.gen(n) return coroutine.wrap(function() for i = 1, n do coroutine.yield(i) end end) end
function M.useGen(n) local s = 0 for v in M.gen(n) do s += v end return s end
function M.multi() return pcall(function() return 1, 2, 3 end) end
function M.typecheck(v)
    local t = type(v)
    if t == "number" then return v + 1
    elseif t == "string" then return #v
    elseif t == "table" then
        local n = 0
        for _ in pairs(v) do n += 1 end
        return n
    elseif t == "function" then return M.typecheck(v())
    end
    error("bad type " .. t, 0)
end
print(M.basic(function() error("e", 0) end)) print(M.withargs(function(a, b) return a + b end, 1, 2), M.withargs(error)) print(M.inline()) print(M.tableerr())
print(M.handler(function() error("q", 0) end)) print(M.assertion(1), (pcall(M.assertion))) print(M.assertcall(function() return 1 end)) print(M.nested()) print(M.retry(function(i) if i < 3 then error("n") end return i end, 5))
print(M.wrap()) print(M.create()) print(M.useGen(3)) print(M.multi()) print(M.typecheck(1), M.typecheck("ab"), M.typecheck({1}), M.typecheck(function() return 1 end), pcall(M.typecheck, nil))
