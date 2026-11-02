-- parameters reassigned through expressions; nothing may shadow them
local mode = 1
local Settings = {Phone = 10, Desktop = 20}
local M = {}
function M.select(size)
    size = if mode == 1 then size or Settings.Phone else size or Settings.Desktop
    return size + 4
end
function M.selectnil(size)
    if mode == 1 then
        size = size or Settings.Phone
    else
        size = size or Settings.Desktop
    end
    return size + 4
end
function M.defaults(a, b, c)
    a = a or 1
    b = b or a + 1
    c = c or b + 1
    return a, b, c
end
function M.clamp(x, lo, hi)
    if x < lo then x = lo end
    if x > hi then x = hi end
    return x
end
function M.branchparam(x, c)
    if c then
        x = x * 2
    else
        x = x + 1
    end
    return x
end
function M.loopparam(n)
    while n > 10 do n = n // 2 end
    return n
end
function M.loopfield(t)
    t = t.next or t
    while t.next do t = t.next end
    return t.v
end
function M.condparam(x)
    if type(x) == "string" then
        x = tonumber(x)
        if not x then return nil end
    end
    return x + 1
end
function M.nestedparam(a, b)
    if a then
        if b then
            a = b
        end
        return a
    end
    return b
end
function M.closureparam(x)
    x = x or 1
    return function() x += 1 return x end
end
function M.tableparam(opts)
    opts = opts or {}
    local size = opts.size or 10
    local name = opts.name or "x"
    return size, name
end
function M.vararg2(a, ...)
    a = a or select("#", ...)
    return a, ...
end
print(M.select(nil), M.select(1), M.selectnil(nil), M.selectnil(2)) print(M.defaults()) print(M.defaults(5, nil, 1)) print(M.clamp(1, 2, 3), M.clamp(9, 2, 3), M.clamp(2.5, 2, 3))
print(M.branchparam(2, true), M.branchparam(2, false), M.loopparam(100), M.loopfield({next = {next = {v = 1}}, v = 2}), M.loopfield({v = 3}), M.condparam("5"), M.condparam("x"), M.condparam(1))
print(M.nestedparam(1, 2), M.nestedparam(1), M.nestedparam(nil, 3), M.closureparam()(), M.closureparam(2)()) print(M.tableparam()) print(M.tableparam({size = 1, name = "y"})) print(M.vararg2(nil, 1, 2)) print(M.vararg2(9))
