-- helpers given constants: the compiler folds them into the body
local M = {}
function M.index(names, seen)
    local function index(name)
        for i, n in ipairs(names) do
            if n == name then
                return i - 1
            end
        end
        error("Missing " .. name, 0)
    end
    local a = index("HIPS")
    local b = index("ROOT")
    return a + b
end
local function has(t, k)
    for _, v in t do
        if v == k then
            return true
        end
    end
    return false
end
local function count(t, want)
    local n = 0
    for _, v in t do
        if v ~= want then continue end
        n += 1
    end
    return n
end
function M.haskey(t) return has(t, "a"), has(t, 2), has(t, true) end
function M.counted(t) return count(t, "x") + count(t, 1) end
function M.isnil(t) return has(t, nil) end
function M.indexfield(data)
    local function index(name)
        for i, n in ipairs(data.names) do
            if n == name then
                return i - 1
            end
        end
        error("Missing " .. name, 0)
    end
    local a = index("A")
    local b = index("B")
    return {a, b}
end
function M.upreturn(x, c)
    local function get() return x end
    local function get2() if c then return x end return 1 end
    return get(), get2()
end
local function center(p, ok)
    if ok then
        return p.x, p.y, p.z
    end
    return 0, 0, 0
end
local function firstpair(t, w)
    for i, v in t do
        if v == w then
            return i, v
        end
    end
    return 0, nil
end
function M.three(p, ok)
    local x, y, z = center(p, ok)
    return x + y + z
end
function M.pair(t, w)
    local i, v = firstpair(t, w)
    print(i, v)
    return i
end
function M.onlyfirst(t, w)
    local i = firstpair(t, w)
    return i
end
print(M.three({x = 1, y = 2, z = 3}, true), M.three({}, false), M.pair({"a"}, "a"), M.pair({}, 1), M.onlyfirst({5}, 5))
local function active(t)
    if t.off then
        return false
    end
    local r = t.r
    return r ~= nil and r.on()
end
function M.chain(t, u)
    if active(t) or active(u) then
        return 1
    end
    return 2
end
function M.stored(t)
    t.a = active(t)
    return t.a
end
local on = {on = function() return true end}
print(M.chain({off = true}, {r = on}), M.chain({}, {}), M.stored({r = on}), M.stored({}), M.stored({off = 1}))
local seen, wins, fired = {}, {}, 0
local function note(k)
    if seen[k] then
        wins.pad = true
    elseif wins[k] then
        wins.pad = false
    else
        return
    end
    fired += 1
end
function M.twice(a, b)
    note(a)
    note(b)
    return fired
end
seen.x = 1
wins.y = 1
print(M.twice("x", "y"), M.twice("z", "q"), wins.pad)
local function entry(t, id)
    for i, e in t do
        if e.id == id then
            return e, i, t
        end
    end
    t.last = id
    return t[1], 1, t
end
function M.first(t, id)
    local e = entry(t, id)
    return e and e.id
end
function M.both(t, id)
    local e, i = entry(t, id)
    return e and e.id, i
end
local function touch(t)
    if t.done then
        return false
    end
    t.done = true
    return true
end
function M.touched(t)
    touch(t)
    touch(t)
    return t.done
end
print(M.first({{id = 1}}, 1), M.first({{id = 2}}, 1), M.both({{id = 1}}, 1), M.touched({}))
local function setup(t)
    if t.off then
        return false
    end
    t.ready = true
    return true
end
function M.laststmt(t, a)
    if a then
        setup(t)
    else
        t.other = 1
    end
end
local x, y = {off = 1}, {}
M.laststmt(x, true)
M.laststmt(y, true)
M.laststmt(x, false)
print(x.ready, y.ready, x.other)
function M.firstcall(data)
    local function index(name)
        for i, n in ipairs(data.names) do
            if n == name then
                return i
            end
        end
        return 0
    end
    local a = index("A")
    print(a)
    local b = index("B")
    print(index ~= nil)
    return a + b
end
print(pcall(M.firstcall, {names = {"B", "A"}}))
local function middle(r)
    return (r[1] + r[3]) / 2, (r[2] + r[4]) / 2
end
function M.seconds(p, q)
    local _, a = middle(p)
    local _, b = middle(q)
    return b - a
end
print(M.seconds({1, 2, 3, 4}, {5, 6, 7, 8}))
print(M.indexfield({names = {"B", "A"}})[1], M.upreturn(5, true)) print(M.upreturn(5, false))
print(M.index({"HIPS", "ROOT"}), pcall(M.index, {"ROOT"})) print(M.haskey({"a", 2})) print(M.haskey({})) print(M.counted({"x", 1, 1, "y"}), M.isnil({}))
