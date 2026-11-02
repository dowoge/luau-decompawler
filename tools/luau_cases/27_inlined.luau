-- local functions with early returns, inlined into the middle of things
local Items = {{Yes = false, Name = "a"}, {Yes = true, Name = "b"}, {Yes = true, Closed = true, Name = "c"}}
local function first(list)
    for _, v in list do
        if not v.Yes or v.Closed then continue end
        return v
    end
    return nil
end
local function find(list, k)
    for i, v in ipairs(list) do
        if v == k then
            return i
        end
    end
    return nil
end
local function position(p)
    if p.kind == "part" then
        return p.pos
    elseif p.kind == "model" then
        local a, b = p.box()
        return a + b
    end
    return 0
end
local function isPad(k, last)
    if string.find(last, "^Pad") ~= nil then
        return k
    end
    return false
end
local M = {}
function M.searchif(list, busy)
    local v
    if not busy then
        v = first(list)
    else
        v = nil
    end
    return v and v.Name or "none"
end
function M.searchthen(list, c)
    local out = "x"
    if c then
        local i = find(list, 2)
        if i then
            out = "found" .. i
        end
    else
        out = "skip"
    end
    return out
end
function M.twosearch(a, b, k)
    local x = find(a, k) or find(b, k)
    if x then
        return x * 2
    end
    return -1
end
function M.posthen(p, q, use)
    local at
    if use then
        at = position(p)
    else
        at = position(q)
    end
    return at + 1
end
function M.nestedpos(p, c, d)
    if c then
        if d then
            local a = position(p)
            print("near", a)
        else
            print("far")
        end
        print("after")
    end
    return position(p)
end
function M.guardchain(k, last, make)
    local on = k.on
    if on then
        print("on")
    elseif isPad(k, last) then
        make()
        return "made"
    end
    if not isPad(k, last) then
        return "plain"
    end
    print("fell")
    return "fell"
end
function M.inloop(list, keys)
    local n = 0
    for _, k in keys do
        local i = find(list, k)
        if i then
            n += i
        else
            n -= 1
        end
    end
    return n
end
function M.loopbreak(lists, k)
    local hit
    for _, l in lists do
        hit = find(l, k)
        if hit then break end
    end
    return hit
end
function M.manyreturns(x, y)
    local r
    if x then
        if y == 1 then
            r = "a"
        elseif y == 2 then
            for i = 1, 3 do
                if i == x then
                    r = "loop" .. i
                    break
                end
            end
            r = r or "none"
        else
            r = "b"
        end
    else
        r = "c"
    end
    return r
end
local Cache = {}
local Who = {Character = {FindChild = function(s, n) return {Parent = s, Health = 5} end}}
local function humanoid()
    local c = Who and Who.Character
    if c then
        local h = Cache[Who]
        if h and h.Parent == c then
            return h
        else
            Cache[Who] = nil
            local found = c:FindChild("Humanoid")
            if found then
                Cache[Who] = found
            end
            return found
        end
    else
        return nil
    end
end
local function name(x) return tostring(x) end
local function shout(x) return string.upper(tostring(x)) end
local function pick(a, b)
    if a then
        return tostring(a)
    end
    return tostring(b)
end
local function nothing(t) t.n = 1 end
local function maybe(a) if a then return 1 end end
function M.alive()
    local h = humanoid()
    return h ~= nil and h.Health > 0
end
function M.called(x) return name(x) .. shout(x) end
function M.picked(a, b) local s = pick(a, b) return s, s end
function M.intable(a) return {name(a), shout(a)} end
function M.inif(a) if name(a) == "1" then return "one" end return shout(a) end
function M.stmt(t) nothing(t) return t.n end
function M.maybeval(a) local v = maybe(a) return v, v end
local function bump(t, by) t.n = (t.n or 0) + by return t.n end
local function field(t, k) return t[k] end
local function put(t, k, v) t[k] = v end
local function scaled(x, f) return x * f, x end
local function say(a, b) print(a, b) end
local function offset(x, by)
    if x > 0 then
        return x + by
    end
    return x - by
end
local function deferred(x)
    local f = function() return x end
    return f
end
function M.constargs(t) bump(t, 2) bump(t, 3) return t.n end
function M.constkey(t) return field(t, "a"), field(t, 1), field(t, "b c") end
function M.constput(t) put(t, "x", 1) put(t, 2, true) return t end
function M.constval(x) return (scaled(x, 2)) end
function M.constcall(a) say(a, "fixed") say("left", a) say(nil, a) end
function M.constbranch(x) return offset(x, 10), offset(-x, 10) end
function M.closurehelper(x) local f = deferred(x) return f() end
function M.iife(list)
    return (function()
        for _, v in list do
            if v == 3 then
                return true
            end
        end
        return false
    end)()
end
function M.iifearg(a, b)
    local x = (function(p, q)
        if p then
            return p + q
        end
        return q
    end)(a, b)
    return x, x
end
function M.iifecheck(t, name)
    local r = {name = name, passed = (function()
        if #t ~= 2 then
            return false
        end
        for i = 2, #t do
            if t[i] <= t[i - 1] then
                return false
            end
        end
        return true
    end)()}
    return r.passed
end
function M.iifecap(n)
    local c = 0
    (function()
        c += n
    end)()
    return c
end
function M.dropped(list, a)
    local function index(x)
        for i, v in ipairs(list) do
            if v == x then
                return i
            end
        end
        return -1
    end
    return index(a) + index(a + 1)
end
function M.dropped2(t)
    local total = 0
    local function add(n)
        if n > 0 then
            total += n
        end
    end
    add(t.a)
    add(t.b)
    return total
end
print(M.dropped({1, 2}, 1), M.dropped({}, 1), M.dropped2({a = 1, b = 2}), M.dropped2({a = -1, b = 0}))
print(M.iife({1, 3}), M.iife({1}), M.iifearg(1, 2)) print(M.iifearg(nil, 2)) print(M.iifecheck({1, 2}, "a"), M.iifecheck({2, 1}, "b"), M.iifecheck({}, "c"), M.iifecap(4))
print(M.constargs({}), M.constkey({a = 1, [1] = 2, ["b c"] = 3})) print(M.constput({}).x, M.constval(4)) M.constcall(1) print(M.constbranch(1)) print(M.closurehelper(9))
print(M.alive(), M.alive(), M.called(1), M.picked(1, 2)) print(M.picked(nil, 2)) print(M.intable(2)[2], M.inif(1), M.inif("b"), M.stmt({})) print(M.maybeval(true)) print(M.maybeval(false))
print(M.searchif(Items), M.searchif(Items, true), M.searchif({})) print(M.searchthen({1, 2}, true), M.searchthen({1}, true), M.searchthen({}, false))
print(M.twosearch({1}, {2}, 2), M.twosearch({1}, {2}, 3), M.twosearch({5}, {}, 5))
local part = {kind = "part", pos = 3} local model = {kind = "model", box = function() return 1, 2 end} print(M.posthen(part, model, true), M.posthen(part, model, false), M.posthen({}, {}, true))
print(M.nestedpos(part, true, true), M.nestedpos(model, true, false), M.nestedpos({}))
print(M.guardchain({on = true}, "x"), M.guardchain({}, "Pad", function() print("mk") end), M.guardchain({on = true}, "Pad1"))
print(M.inloop({1, 2}, {2, 3, 1}), M.loopbreak({{1}, {2, 3}}, 3), M.loopbreak({{1}}, 9), M.manyreturns(1, 1), M.manyreturns(2, 2), M.manyreturns(9, 2), M.manyreturns(1, 3), M.manyreturns())
