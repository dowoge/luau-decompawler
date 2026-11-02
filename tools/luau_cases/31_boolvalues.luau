-- values of comparisons joined with and/or: the compiler loads the true or
-- false first, and tests with the jump going straight to the end
local M = {}
local fn, mod = nil, {captures = function() return "c" end}
function M.setfn(f) fn = f end
function M.a1() return fn ~= nil and fn() == true or mod.captures() end
function M.a2(x) return x == 1 or mod.captures() end
function M.a3(x) return x == 1 and mod.captures() end
function M.a4(x, y) return x == 1 and y == 2 or mod.captures() end
function M.a5(x, y) return x == 1 or y == 2 or mod.captures() end
function M.a6(x, y, z) return x == 1 and y == 2 and z == 3 or mod.captures() end
function M.a7(x, y) return (x == 1 or y == 2) and mod.captures() end
function M.a8(x, y) return x == 1 and y == 2 and mod.captures() end
function M.a9(x, y, z) return (x == 1 and y == 2 or z == 3) and mod.captures() end
function M.b1(x, y)
    local r = x == 1 and y == 2 or mod.captures()
    print(r)
    return r
end
function M.b2(x, y)
    local r = x < y or x == 9
    return r
end
function M.b3(t, x)
    t.v = x ~= nil and x > 1 or mod.captures()
    return t.v
end
function M.b4(x, y)
    print(x == 1 and y == 2 or mod.captures(), y)
end
function M.b5(x)
    return x ~= nil and x.on == true
end
function M.b6(x, y)
    return not (x == 1) and y == 2 or mod.captures()
end
function M.b7(x, y, z)
    if (x == 1 and y == 2 or z == 3) and mod.captures() then
        return 1
    end
    return 2
end
function M.b8(x, y)
    return (x == nil or x == y) and mod.captures() or false
end
local send = function(...) return select("#", ...), ... end
function M.c1(x) return send("everyone", x == true) end
function M.c2(x) local r = send("everyone", x == true) return r end
function M.c3() return fn ~= nil and fn() end
function M.c4() return fn == nil or fn() end
function M.c5() return mod.x ~= nil and mod.x.y() end
function M.c6(t, x) table.insert(t, x ~= nil) return #t end
function M.d1(x, free)
    return if free == true then 0 else x
end
function M.d2(a, b, c)
    return if a < b then a else c
end
function M.d3(t, k, d)
    local v = t[k]
    return if v == nil then d else v
end
function M.d4(a, b)
    if a then return 1 end
    return b
end
function M.e1(panel)
    local b = panel and panel.kind == "donate" and panel.buttons.donate
    if b ~= nil then
        print(b)
        return true
    end
    return false
end
function M.e2(p)
    local v = p
    local b = v and v.name == "x" and v.tags
    return b, v == p
end
function M.f1(a, name)
    local v = a and a:find(name)
    if v ~= nil then
        print(v)
    end
end
function M.f2(pos, svc)
    local p = pos or svc:location()
    return p.x + p.y
end
function M.f3(ok, r)
    local v = ok and r.find(1)
    if v then
        return v.name
    end
    return nil
end
function M.f4(a)
    return coroutine.wrap(function()
        print(a)
    end)()
end
function M.f5(a)
    coroutine.wrap(function()
        print(a)
    end)()
    return (select("#", function()
        return a
    end, a))
end
local function first(a, b)
    return a
end
local function three(...)
    return select("#", ...)
end
function M.g1(x)
    print(three(1, 2, first(true, if x == tostring(5) then `<{x}>` else 1)))
end
function M.g2(a, b, c, d)
    local x, y
    if a < b then
        x = b < c
        y = c < d
    else
        x = c < b
        y = d < c
    end
    print(x)
    return x, y
end
print(M.g2(1, 2, 3, 4), M.g2(2, 1, 0, 3))
M.g1("5")
M.g1("6")
M.f1({find = function(_, n) return n end}, "x")
print(M.f2(nil, {location = function() return {x = 1, y = 2} end}), M.f3(true, {find = function() return {name = "n"} end}), M.f3(false))
M.f4(1)
print(M.f5(2))
print(M.e1({kind = "donate", buttons = {donate = 1}}), M.e1({kind = "x"}), M.e1(nil), M.e2({name = "x", tags = 1}))
print(M.c1(true), M.c2(false), M.c3(), M.c4(), M.c5(), M.c6({}, 1))
print(M.d1(5, true), M.d1(5), M.d2(1, 2, 3), M.d2(2, 1, 3), M.d3({a = 1}, "a", 2), M.d3({}, "a", 2), M.d4(1, 2), M.d4(false, 2))
for _, x in {1, 2, 3} do
    for _, y in {1, 2, 3} do
        print(M.a2(x), M.a3(x), M.a4(x, y), M.a5(x, y), M.a6(x, y, y), M.a7(x, y), M.a8(x, y), M.a9(x, y, 3))
        print(M.b1(x, y), M.b2(x, y), M.b3({}, x), M.b5({on = x == 1}), M.b6(x, y), M.b7(x, y, x), M.b8(x, y))
        M.b4(x, y)
    end
end
print(M.a1())
M.setfn(function() return true end)
print(M.a1())
M.setfn(function() return false end)
print(M.a1())
