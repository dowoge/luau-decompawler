-- values built from several paths, returned directly
local Run = {}
function Run.studio() return false end
function Run.now() return 5 end
local Config = {Force = true, Limit = 10}
local M = {}
function M.flag() return Run.studio() and Config.Force or Run.now() >= 3 and Run.now() <= 8 end
function M.flag2(a, b) return a > 0 and b > 0 or a < 0 and b < 0 end
function M.orand(a, b, c) return (a or b) and c end
function M.andor(a, b, c) return a and b or c end
function M.triple(a, b, c) return a or b or c end
function M.triple2(a, b, c) return a and b and c end
function M.cmpchain(a) return a >= 1 and a <= 9 end
function M.cmpor(a) return a < 1 or a > 9 end
function M.notchain(a, b) return not a and not b end
function M.notor(a, b) return not a or not b end
function M.callchain(f, a) return f(a) and f(a + 1) or f(a + 2) end
function M.indexchain(t) return t.a and t.a.b or t.c end
function M.chained3(a, b, c, d) return a and b or c and d end
function M.chained4(a, b, c, d) return (a or b) and (c or d) end
function M.withnot(a, b) return a and not b end
function M.guard(a) if not a then return nil end return a + 1 end
function M.guard2(a, b) if a then return a end return b end
function M.guard3(a) if a == 1 then return "x" end return "y" end
function M.guard4(a) if a > 0 then return a * 2 else return a * 3 end end
function M.guard5(a, b)
    if a == nil then return false end
    if b == nil then return false end
    return a == b
end
function M.boolret(a) if a then return true end return false end
function M.boolret2(a) if a then return true else return false end end
function M.nilret(a) if a then return a end end
function M.local1(a, b) local x = a and b or 0 return x, x end
function M.local2(a, b) local x = (a or b) and 1 return x, x end
function M.field(t, a) t.v = a and 1 or 2 return t.v end
function M.argval(f, a, b) f(a and b or 0) return f(a or b) end
print(M.flag(), M.flag2(1, 1), M.flag2(1, -1), M.flag2(-1, -1), M.orand(nil, 1, 2), M.orand(nil, nil, 2), M.andor(1, 2, 3), M.andor(nil, 2, 3), M.andor(1, nil, 3))
print(M.triple(nil, nil, 3), M.triple(nil, 2, 3), M.triple2(1, 2, 3), M.triple2(1, nil, 3), M.cmpchain(5), M.cmpchain(0), M.cmpor(5), M.cmpor(0), M.notchain(nil, nil), M.notchain(1, nil), M.notor(1, 2), M.notor(nil, 2))
local id = function(x) return x % 2 == 0 and x or nil end print(M.callchain(id, 1), M.callchain(id, 2), M.indexchain({a = {b = 1}}), M.indexchain({a = {}, c = 2}), M.indexchain({c = 3}))
print(M.chained3(1, 2, 3, 4), M.chained3(nil, 2, 3, 4), M.chained3(nil, 2, nil, 4), M.chained4(nil, 2, nil, 4), M.chained4(nil, nil, 3, 4), M.withnot(1, nil), M.withnot(1, 2), M.withnot(nil, 2))
print(M.guard(1), M.guard(nil), M.guard2(1, 2), M.guard2(nil, 2), M.guard3(1), M.guard3(2), M.guard4(2), M.guard4(-2), M.guard5(nil, 1), M.guard5(1, nil), M.guard5(1, 1), M.boolret(1), M.boolret(nil), M.boolret2(1), M.boolret2(nil), M.nilret(1), M.nilret(nil))
print(M.local1(1, 2)) print(M.local1(nil, 2)) print(M.local2(nil, 1)) print(M.local2(nil, nil)) print(M.field({}, true), M.field({}, false), M.argval(function(x) return x end, 1, 2))
