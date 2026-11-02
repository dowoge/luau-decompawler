-- a call's result reaches a lower register through a temporary only when that
-- register already holds something: a variable, or a helper's result
local M = {}
local function make(kind)
    return {kind = kind}
end
M.make = make
function M.changed(t, k)
    local v = t[k]
    if v then
        return v.kind
    end
    v = M.make("event")
    t[k] = v
    return v.kind
end
function M.param(x)
    print(x)
    x = M.make("p")
    return x.kind
end
function M.loop(n)
    local v = M.make("a")
    for i = 1, n do
        v = M.make(v.kind .. i)
    end
    return v.kind
end
function M.arms(c)
    local v
    if c then
        v = M.make("then")
    else
        v = M.make("else")
    end
    return v.kind
end
local function attach(parent, position)
    local a = M.make("attachment")
    a.position = position
    a.parent = parent
    return a
end
function M.at(team, p, off)
    local r = attach(team, p - off)
    return r.position, r.parent
end
function M.twice(team)
    local a = attach(team, 1)
    local b = attach(team, 2)
    return a.position + b.position
end
local function bits(v)
    local n = math.floor(v)
    local f = v - n
    if f > 0.5 then
        n += 1
    end
    return n
end
function M.round(x)
    return bits(math.abs(x)) + bits(x * 2)
end
function M.reused(t)
    local first = M.make("a")
    local second = first.kind
    local third = M.make("b")
    return function()
        return second, third.kind
    end
end
function M.topmost(t, k)
    local v = t[k]
    print(v)
    local x = M.make("e")
    t[k] = x
    return x.kind
end
function M.twodead(t, k)
    local v = t[k]
    local w = t[v]
    print(w)
    local x = M.make("e")
    print(x.kind, v)
    local y = M.make("f")
    return y.kind, x.kind
end
function M.afterclosure(a)
    local function mk(n)
        local c = 0
        return function(x)
            c += 1
            return x + n * c
        end
    end
    local f = mk(4)
    local v = if a then tostring(1) .. "a" else string.rep("q", 2)
    v = string.rep("hello", 2)
    return f(1), v
end
function M.forlist(...)
    local out = M.make("list")
    for _, v in {...} do
        out[v] = true
    end
    for i, v in {3, 2} do
        out[i] = v
    end
    for k, v in {a = 1} do
        out[k] = v
    end
    return out[1], out[2], out.a, out.x
end
function M.declared(a, b, c)
    local v
    if b - a >= 0 then
        v = b
    else
        v = a
    end
    if c - v >= 0 then
        return v
    end
    return c
end
function M.counter(a)
    local n = 0
    if a then
        n = 2
    else
        n = 3
    end
    return n
end
function M.cleanup(t)
    table.clear(t)
    t = nil
end
function M.lastword(x)
    local s = M.make("q")
    print(s.kind)
    s = nil
end
function M.ctor(n)
    local v = {frame = {dt = 0, current = nil, times = table.create(n)}, active = {}}
    v.extra = n
    return #v.frame.times
end
function M.ctor2(c)
    local v = {on = c == "always"}
    return v.on
end
function M.ctor3(c)
    local v = {text = if c then M.make("a") else M.make("b")}
    return v.text.kind
end
local function n(args, i)
    return math.floor(tonumber(args[i]) or 0)
end
local self_ = {category = function(_, i) return {icon = i} end}
function M.methodarg(args)
    local c = self_:category(n(args, 1))
    return c and c.icon
end
function M.chain(args)
    local size = self_
        :category(
            n(args, 1)
        )
        .icon
    return size + 1
end
function M.lastarg(args)
    return tostring(n(args, 1))
end
function M.find(root, bone)
    local b = root and root:find(bone, true)
    return if b ~= nil and b.ok then b else nil
end
local state = {}
function M.pairstore()
    state.x, state.y = 0, 1
    return state.x + state.y
end
function M.fromfield(b)
    local body = b.body
    body.a, body.b = M.make("p"), M.make("q")
    b.state = 1
    return body.a.kind .. body.b.kind
end
function M.mixed(t, i, v)
    t.a, t[i], t[1] = v, v + 1, "s"
    return t.a, t[i], t[1]
end
function M.separate(t, p, q)
    t.a = p
    t.b = q
    t.c = p
    return t.c
end
function M.ctor4(n, c)
    local t = {a = n, b = M.make("x"), c = c == 1, d = if c then 1 else 2}
    t.e = 1
    return t
end
print(M.ctor4(1, 1).c, M.ctor4(2, 2).d)
print(M.pairstore(), M.fromfield({body = {}}), M.mixed({}, 5, 2), M.separate({}, 1, 2))
print(M.methodarg({"3"}), M.chain({"4"}), M.lastarg({"5"}))
print(M.find({find = function() return {ok = 1, v = 2} end}, "x").v, M.find(nil, "x"))
print(M.declared(1, 2, 3), M.declared(3, 2, 1), M.counter(true), M.counter(false))
M.cleanup({})
M.lastword(1)
print(M.ctor(2), M.ctor2("always"), M.ctor3(true), M.ctor3(false))
print(M.forlist("x", "y"))
print(M.afterclosure(true), M.afterclosure(false))
print(M.reused({})())
print(M.topmost({}, 1), M.twodead({}, 1))
print(M.changed({}, 1), M.changed({{kind = "k"}}, 1), M.param(1), M.loop(2), M.arms(true), M.arms(false))
print(M.at("t", 3, 1))
print(M.twice("t"), M.round(1.7), M.round(-2.2))
