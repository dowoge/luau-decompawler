-- values that closures and later writes must not lose
local M = {}
function M.tablecap() local t = {1, 2, 3} return function() return #t end end
function M.hashcap() local t = {a = 1, b = 2} return function() return t.b end end
function M.varargcap(...) local t = {...} return function() return #t end end
function M.mixedcap(x) local t = {x, y = x} return function() return t[1], t.y end end
function M.callcap(f) local t = {f()} return function() return #t end end
function M.nestedcap(x) local t = {p = {q = x}} return function() return t.p.q end end
function M.emptycap() local t = {} return function() t.x = 1 return t.x end end
function M.upvalread()
    local n = 1
    local function bump() n += 1 end
    local a = n
    bump()
    return a, n
end
function M.upvalarg()
    local n = 1
    local function bump() n += 1 return n end
    return n, bump(), n
end
function M.upvalorder()
    local n = 1
    local function bump() n += 1 end
    local t = {}
    local before = n
    bump()
    t.before = before
    t.after = n
    return t.before, t.after
end
function M.upwrite()
    local n = 1
    local function set(v) n = v end
    local old = n
    set(5)
    return old
end
local counter = 0
function M.globalup()
    local a = counter
    counter += 1
    return a, counter
end
function M.tempswap(a, b)
    local c = a
    a = b
    b = c
    return a, b
end
function M.stale(a)
    local b = a
    a = a + 1
    return a, b
end
function M.stalefield(t)
    local v = t.x
    t.x = v + 1
    return v, t.x
end
function M.loopstale(n)
    local prev = 0
    local cur = 1
    for _ = 1, n do
        local next_ = prev + cur
        prev = cur
        cur = next_
    end
    return prev, cur
end
function M.threeway(a, b, c) a, b, c = c, a, b return a, b, c end
function M.partswap(a, b) a, b = b, a + 1 return a, b end
function M.calledswap(f, a, b) a, b = f(b), f(a) return a, b end
function M.tableswap(t, i, j) t[i], t[j] = t[j], t[i] return t[i], t[j] end
function M.fieldlocal(t, a) t.x, a = a, t.x return t.x, a end
function M.loopswap(t, s)
    for _, x in ipairs(t) do
        s, x = x, s + x
        print(s, x)
    end
    return s
end
function M.loopswap2(n)
    local a, b = 1, 2
    for i = 1, n do
        a, b = b, a + b + i
    end
    return a, b
end
function M.threeloop(a, b, c, n)
    while n > 0 do
        a, b, c = c, a + 1, b * 2
        n -= 1
    end
    return a, b, c
end
print(M.loopswap({1, 2}, 3), M.loopswap2(3)) print(M.threeloop(1, 2, 3, 2))
print(M.tablecap()(), M.hashcap()(), M.varargcap(1, 2)()) print(M.mixedcap(1)()) print(M.callcap(function() return 1, 2 end)(), M.nestedcap(3)(), M.emptycap()())
print(M.upvalread()) print(M.upvalarg()) print(M.upvalorder()) print(M.upwrite()) print(M.globalup()) print(M.globalup())
print(M.tempswap(1, 2)) print(M.stale(1)) print(M.stalefield({x = 1})) print(M.loopstale(5)) print(M.threeway(1, 2, 3)) print(M.partswap(1, 2))
print(M.calledswap(function(x) return x * 2 end, 1, 2)) print(M.tableswap({1, 2}, 1, 2)) print(M.fieldlocal({x = 1}, 2))
