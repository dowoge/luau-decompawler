-- side effects must happen in the order the source says
local log = {}
local T = {}
function T.t(name, v) table.insert(log, name) return v end
local function flush()
    local s = table.concat(log, " ")
    table.clear(log)
    return s
end
local M = {}
function M.gt() return T.t("a", 2) > T.t("b", 1) end
function M.ge() return T.t("a", 2) >= T.t("b", 1) end
function M.lt() return T.t("a", 2) < T.t("b", 1) end
function M.le() return T.t("a", 2) <= T.t("b", 1) end
function M.gtif() if T.t("a", 2) > T.t("b", 1) then return "y" end return "n" end
function M.ltif() if T.t("a", 2) < T.t("b", 1) then return "y" end return "n" end
function M.notgt() if not (T.t("a", 2) > T.t("b", 1)) then return "y" end return "n" end
function M.eq() return T.t("a", 2) == T.t("b", 1) end
function M.plus() return T.t("a", 2) + T.t("b", 1) end
function M.minus() return T.t("a", 2) - T.t("b", 1) end
function M.concat() return T.t("a", "x") .. T.t("b", "y") .. T.t("c", "z") end
function M.args() return math.max(T.t("a", 2), T.t("b", 1), T.t("c", 3)) end
function M.index() local o = {x = 1} return T.t("a", o)[T.t("b", "x")] end
function M.store() local o = {} T.t("a", o)[T.t("b", "x")] = T.t("c", 1) return o.x end
function M.tbl() local o = {T.t("a", 1), T.t("b", 2), k = T.t("c", 3)} return #o, o.k end
function M.nestcall() return T.t("a", math.abs)(T.t("b", -1)) + T.t("c", 1) end
function M.reversed()
    local a = T.t("a", 1)
    local b = T.t("b", 2)
    return b - a
end
function M.reversedcall()
    local a = T.t("a", 1)
    local b = T.t("b", 2)
    return math.max(b, a)
end
function M.outoforder()
    local a = T.t("a", 1)
    local b = T.t("b", 2)
    local c = T.t("c", 3)
    return c, a, b
end
function M.kept()
    local a = T.t("a", 1)
    T.t("mid")
    return a
end
function M.keptif(c)
    local a = T.t("a", 1)
    if c then T.t("mid") end
    return a
end
function M.andor(c) return c and T.t("a", 1) or T.t("b", 2) end
function M.orchain(a, b) return a or b or T.t("c", 3) end
function M.andchain(a, b) return a and b and T.t("c", 3) end
function M.mixedchain(a, b, c) return (a and T.t("x", b)) or (not a and T.t("y", c)) end
function M.ifargs(c) return math.max(if c then T.t("a", 1) else T.t("b", 2), if c then T.t("c", 3) else T.t("d", 4)) end
function M.ifchain(c) return (if c then T.t("a", 1) else T.t("b", 2)) + (if c then T.t("c", 3) else T.t("d", 4)) end
function M.leftofif(c) return T.t("a", "x") .. (if c then "y" else "z") end
function M.orleft(a, c) return (a or "x") .. (c and "y" or "z") end
function M.tableif(c) return {T.t("a", 1), if c then 2 else 3, T.t("b", 4)} end
for _, name in ipairs({"gt", "ge", "lt", "le", "gtif", "ltif", "notgt", "eq", "plus", "minus", "concat", "args", "index", "store", "tbl", "nestcall", "reversed", "reversedcall", "outoforder", "kept"}) do
    print(name, M[name]())
    print(flush())
end
print(M.keptif(true), flush()) print(M.keptif(false), flush())
print(M.andor(true), flush()) print(M.andor(false), flush()) print(M.orchain(nil, nil), flush()) print(M.orchain(1, 2), flush())
print(M.andchain(1, 2), flush()) print(M.andchain(nil, 2), flush()) print(M.mixedchain(1, nil, 3), flush()) print(M.mixedchain(nil, 2, 3), flush())
print(M.ifargs(true), flush()) print(M.ifargs(false), flush()) print(M.ifchain(true), flush()) print(M.ifchain(false), flush()) print(M.leftofif(true), flush()) print(M.orleft(nil, nil), flush()) print(#M.tableif(true), flush())
