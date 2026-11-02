local M = {}
function M.counter() local n = 0 return function() n += 1 return n end end
function M.constcap(a) return function() return a end end
function M.twocap(a) local b = 1 return function() b += a return b end end
function M.nocap() return function() return 1 end end
function M.shared()
    local n = 0
    local function inc() n += 1 end
    local function get() return n end
    return inc, get
end
function M.forward()
    local a
    local function f() return a end
    a = 5
    return f()
end
function M.forward2(x)
    local cb
    local t = {run = function() return cb and cb(x) end}
    cb = function(v) return v + 1 end
    return t.run()
end
function M.forward3()
    local g
    local function f(n) if n == 0 then return 0 end return g(n - 1) end
    function g(n) return f(n) end
    return f(3)
end
function M.selfrec(n)
    local function fact(k) if k <= 1 then return 1 end return k * fact(k - 1) end
    return fact(n)
end
function M.assignrec(n)
    local fib
    fib = function(k) if k < 2 then return k end return fib(k - 1) + fib(k - 2) end
    return fib(n)
end
function M.iife(a) return (function(b) return a + b end)(1) end
function M.nest3(a)
    return function(b)
        return function(c)
            a += 1
            return a + b + c
        end
    end
end
function M.paramcap(a, b) a = a or 1 return function() return a + b end end
function M.mutparam(a) local function f() a += 1 end f() return a end
function M.brancheddecl(c)
    local f
    if c then
        f = function() return "a" end
    else
        local v = 1
        f = function() return v end
    end
    return f()
end
function M.reassigned()
    local x = 1
    local f = function() return x end
    x = 2
    local g = function() return x end
    return f(), g()
end
function M.doblock()
    local r
    do
        local a = 1
        r = function() return a end
    end
    do
        local a = 2
        r = function() return r, a end
    end
    return select(2, r())
end
function M.methodcap(o) function o.f() return o end return o.f() == o end
function M.upvalset() local a = 1 local function s(v) a = v end s(2) return a end
function M.deepset() local a = 1 local function o() local function i() a = 3 end i() end o() return a end
function M.argclosure(f, x) return f(function(v) return v + x end) end
function M.tableclosure(x) return {a = function() return x end, b = function() x += 1 end, 1, function() return 2 end} end
function M.varcap(...) local a = {...} return function() return #a end end
function M.loopvarcap(n)
    local last
    for i = 1, n do
        local v = i * 2
        last = function() return v end
    end
    return last and last()
end
function M.perpass(list)
    local fs = {}
    for _, item in list do
        local Art = item.Art
        local Page
        Page = item.Page
        table.insert(fs, function() return Art, Page end)
    end
    return fs
end
function M.perpass2(list)
    local fs = {}
    for i, item in list do
        if item.on then
            local Art = item.Art
            fs[i] = function() Art += 1 return Art end
        end
    end
    return fs
end
function M.perpass3(n)
    local fs = {}
    local i = 0
    while i < n do
        i += 1
        local v
        v = i * 2
        fs[i] = function() return v end
    end
    return fs
end
function M.carried(n)
    local fs, last = {}, nil
    for i = 1, n do
        local prev = last
        last = i
        fs[i] = function() return prev, last end
    end
    return fs
end
local pp = M.perpass({{Art = 1, Page = 2}, {Art = 3, Page = 4}}) print(pp[1]()) print(pp[2]())
local p2 = M.perpass2({{on = true, Art = 1}, {on = true, Art = 5}}) print(p2[1](), p2[2](), p2[1]())
local p3 = M.perpass3(2) print(p3[1](), p3[2]()) local cr = M.carried(2) print(cr[1]()) print(cr[2]())
print(M.counter()(), M.constcap(2)(), M.twocap(2)(), M.nocap()()) local inc, get = M.shared() inc() print(get())
print(M.forward(), M.forward2(1), M.forward3(), M.selfrec(4), M.assignrec(6), M.iife(1), M.nest3(1)(2)(3), M.paramcap(nil, 2)(), M.mutparam(1))
print(M.brancheddecl(true), M.brancheddecl(false), M.reassigned()) print(M.doblock(), M.methodcap({}), M.upvalset(), M.deepset())
print(M.argclosure(function(f) return f(1) end, 2), M.tableclosure(1).a(), M.varcap(1, 2)(), M.loopvarcap(3))
