local M = {}
function M.arith(a, b) return a + b * 2 - (a - b) / 3 end
function M.powers(a) return a ^ 2 + 2 ^ a, -a ^ 2 end
function M.modulo(a, b) return a % b, a // b, a % 5, 7 // a end
function M.unary(a, t) return -a, not a, #t, - -a, not not a end
function M.compare(a, b) return a < b, a <= b, a > b, a >= b, a == b, a ~= b end
function M.constcmp(a) return a < 3, 3 < a, a == 3, a ~= "x", a == nil, a ~= true end
function M.andv(a, b) return a and b end
function M.orv(a, b) return a or b end
function M.ternary(c, a, b) return c and a or b end
function M.andconst(a) return a and 1 end
function M.orconst(a) return a or "default" end
function M.chain(a, b, c) return a and b and c, a or b or c end
function M.mix(a, b, c) return (a and b) or c, a and (b or c), (a or b) and c end
function M.notand(a, b) return not a and b, not (a and b), not (a or b) end
function M.ifexpr(a) return if a then 1 else 2 end
function M.ifchain(a) return if a == 1 then "one" elseif a == 2 then "two" else "many" end
function M.ifnested(a, b) return if a then (if b then 1 else 2) else 3 end
function M.ifcall(a) local x = if a > 0 then math.sqrt(a) else 0 return x + 1 end
function M.concat(a, b) return a .. b, a .. " " .. b, "<" .. a .. ">" end
function M.interp(a, b) return `{a}`, `x={a} y={b}`, `{a}{b}!` end
function M.index(t, k) return t.a, t[k], t.a.b, t[1], t[k].c, t["with space"] end
function M.call(f, a) return f(), f(a), f(a, a), (f(a)) end
function M.method(o, a) return o:m(), o:m(a), o.f:m(a) end
function M.vararg(...) return ..., select("#", ...) end
function M.varfirst(...) local a, b = ... return b, a end
function M.tablelen(...) return #{...} end
function M.paren(f) return (f()), f() end
function M.bigconst() return 1e300, 0.1, -0.5, 1/0, 123456789012, "a\nb\t\"c\"\\" end
function M.bools() return true, false, nil end
function M.mathfast(a) return math.floor(a), math.abs(a), math.max(a, 1), math.min(1, 2, a), bit32.band(a, 255) end
function M.typecheck(a) return type(a), typeof(a), tostring(a), tonumber(a) end
function M.nested(a) return ((a + 1) * (a - 1)) / ((a + 2) * 2) end
function M.strmeth(s) return s:upper(), s:sub(1, 2), ("x"):rep(3), #s end
function M.cmpvalue(a, b) local x = a == b local y = a < b return x, y end
function M.andcmp(a, b) return a > 0 and b > 0, a > 0 or b > 0 end
function M.notcmp(a, b) return not (a == b), not (a < b) end
print(M.arith(5, 2)) print(M.powers(2)) print(M.modulo(7, 2)) print(M.unary(3, {1}))
print(M.compare(1, 2)) print(M.constcmp(3)) print(M.andv(1, 2), M.orv(nil, 2), M.ternary(true, 1, 2))
print(M.andconst(1), M.orconst(nil), M.chain(1, 2, 3)) print(M.mix(1, nil, 3)) print(M.notand(1, 2))
print(M.ifexpr(true), M.ifchain(2), M.ifnested(true, false), M.ifcall(4)) print(M.concat("a", "b"))
print(M.interp(1, 2)) print(select(3, M.index({a = {b = 1}, [1] = 2, k = {c = 3}, ["with space"] = 4}, "k")))
print(M.call(function(...) return ... end, 1)) print(M.method({m = function(s, x) return x end, f = {m = function(s, x) return x end}}, 1))
print(M.vararg(1, 2)) print(M.varfirst(1, 2)) print(M.tablelen(1, 2)) print(M.paren(function() return 1, 2 end))
print(M.bigconst()) print(M.bools()) print(M.mathfast(3.5)) print(M.typecheck(1)) print(M.nested(3)) print(M.strmeth("abc"))
print(M.cmpvalue(1, 2)) print(M.andcmp(1, 0)) print(M.notcmp(1, 2))
