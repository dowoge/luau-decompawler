-- long and/or chains, in returns, locals, arguments and loop headers
local M = {}
local function is(v, n) return v.kind == n end
function M.four(v) return v:IsA("I") or v:IsA("N") or v:IsA("S") or v:IsA("B") end
function M.fourand(v) return v:IsA("I") and v:IsA("N") and v:IsA("S") and v:IsA("B") end
function M.fourlocal(v)
    local x = v:IsA("I") or v:IsA("N") or v:IsA("S") or v:IsA("B")
    return x, x
end
function M.fourarg(v, f) return f(v:IsA("I") or v:IsA("N") or v:IsA("S")) end
function M.five(a, b, c, d, e) return a or b or c or d or e end
function M.fiveand(a, b, c, d, e) return a and b and c and d and e end
function M.mixedlong(a, b, c, d) return a and b or c and d or false end
function M.iter(t, u) local n = 0 for _, v in ipairs(t or u:list()) do n += v end return n end
function M.iter2(t, u) local n = 0 for k in pairs(t or u or {}) do n += 1 end return n end
function M.iter3(t, c) local n = 0 for _, v in (c and ipairs or pairs)(t) do n += v end return n end
function M.iter4(t) local n = 0 for i = 1, t.n or #t do n += 1 end return n end
function M.callor(f, a, b) return f(a or b, b or a) end
function M.callmeth(o, a, b) return o:m(a or b):m(b and a) end
function M.tableor(a, b) return {a or b, x = b or a, [a or "k"] = 1} end
function M.indexor(t, a, b) return t[a or b], (t.x or t.y).z end
function M.storeor(t, a, b) t[a or b] = b or a return t end
function M.concator(a, b) return "<" .. (a or "x") .. "," .. (b or "y") .. ">" end
function M.arithor(a, b) return (a or 1) + (b or 2) * (a or 3) end
function M.cmpor(a, b) return (a or 0) > (b or 0) end
function M.notor(a, b) return not (a or b), not (a and b) end
function M.ifor(a, b) return if a or b then "y" else "n" end
function M.nestor(a, b, c) return (a or (b and c)) or (b or c) end
print(M.four({IsA = function(_, n) return n == "S" end}), M.fourand({IsA = function() return 1 end}), M.fourlocal({IsA = function(_, n) return n == "B" end}))
print(M.fourarg({IsA = function(_, n) return n == "N" end}, tostring), M.five(nil, nil, nil, nil, 5), M.fiveand(1, 2, 3, 4, 5), M.mixedlong(nil, 1, 1, 2), M.mixedlong(nil, nil, nil, 2))
print(M.iter({1, 2}), M.iter(nil, {list = function() return {3} end}), M.iter2(nil, {1, 2}), M.iter2(), M.iter3({4}, true), M.iter3({5}), M.iter4({n = 2}), M.iter4({1}))
print(M.callor(function(a, b) return a, b end, nil, 2)) local o = {m = function(s, x) return s end} print(M.callmeth(o, 1) == o) local t = M.tableor(nil, 2) print(t[1], t.x, t.k)
print(M.indexor({1, x = {z = 2}}, nil, 1)) print(M.storeor({}, nil, 3)[3], M.concator(nil, "b"), M.arithor(nil, nil), M.cmpor(1), M.cmpor(nil, 1)) print(M.notor(nil, 1)) print(M.ifor(nil, 1), M.ifor(), M.nestor(nil, 1, 2), M.nestor(nil, nil, 2))
