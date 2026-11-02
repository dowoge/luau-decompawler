-- values computed before a branch and read inside it
local log = {}
local T = {}
function T.t(name, v) table.insert(log, name) return v end
local function flush()
    local s = table.concat(log, " ")
    table.clear(log)
    return s
end
local M = {}
function M.signed(self, target, config)
    local mag = T.t("mag", math.abs(target))
    if target < 0 then
        mag = -mag
    end
    return mag + 1
end
function M.signed2(target)
    local v = T.t("v", 5)
    local r
    if target then
        r = v
    else
        r = -v
    end
    return r
end
function M.bothread(c)
    local v = T.t("v", 5)
    return if c then v + 1 else v - 1
end
function M.bothread2(c)
    local v = T.t("v", 5)
    return c and v + 1 or v - 1
end
function M.cond(c)
    local v = T.t("v", 5)
    local w = T.t("c", c)
    return if w then v else 0
end
function M.upval(c)
    local n = 1
    local function bump() n += 1 return n end
    local a = bump()
    return if c then a + n else a - n
end
function M.tablearm(c)
    local t = {T.t("a", 1)}
    return if c then #t else t[1] + 1
end
function M.fieldarm(o, c)
    local v = o.x
    if c then
        return v
    end
    return v + 1
end
function M.negated(x, c)
    local y = T.t("y", x * 2)
    y = if c then y else -y
    return y
end
local function arms(a, b, c)
    return if not a then c - b % 7 else c % 3 + c - 1
end
function M.inlinedarms(x, y)
    return arms(x, y, 5), arms(not x, y, 7)
end
function M.constarms(c)
    local k = 3
    local v = T.t("v", 4)
    return if c then k + v else k - v
end
print(M.inlinedarms(true, 2)) print(M.inlinedarms(false, 2)) print(M.constarms(true), M.constarms(false), flush())
print(M.signed(nil, 3), flush()) print(M.signed(nil, -3), flush()) print(M.signed2(true), flush()) print(M.signed2(false), flush()) print(M.bothread(true), flush()) print(M.bothread(false), flush())
print(M.bothread2(true), flush()) print(M.bothread2(false), flush()) print(M.cond(true), flush()) print(M.cond(false), flush()) print(M.upval(true), M.upval(false), flush())
print(M.tablearm(true), M.tablearm(false), flush()) print(M.fieldarm({x = 1}, true), M.fieldarm({x = 1}, false), M.negated(2, true), M.negated(2, false), flush())
