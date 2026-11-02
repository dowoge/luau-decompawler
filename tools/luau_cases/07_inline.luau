-- what -O2 does to small local functions, constants and short loops
local K = 10
local NAME = "item"
local function double(x) return x * 2 end
local function add(a, b) return a + b end
local function pick(a, b) if a then return a end return b end
local function isodd(n) return n % 2 == 1 end
local function clamp(v, lo, hi) if v < lo then return lo elseif v > hi then return hi end return v end
local function noop() end
local function pair(a) return a, a end
local function greet(n) return "hello " .. n end
local M = {}
function M.a(x) return double(x) + 1 end
function M.b(x, y) return add(x, double(y)) end
function M.c(x) return pick(x, K) end
function M.d(x) if isodd(x) then return "odd" end return "even" end
function M.e(x) return clamp(x, 0, K) end
function M.f(x) noop() return x end
function M.g(x) local a, b = pair(x) return a + b end
function M.h(x) return greet(x), NAME .. "s" end
function M.i(x) return K * 2 + x, K + 1 end
function M.j() local s = 0 for i = 1, 3 do s += i end return s end
function M.k(x) local s = 0 for i = 1, 4 do s += x * i end return s end
function M.l(t) for i = 1, 2 do print(t[i]) end end
function M.m() return 2 * 3 + 4, "a" .. "b", not true, #"abc", 10 / 2 end
function M.n(x) if true then return x end return 1 end
function M.o(x) if false then return 1 else return x end end
function M.p(x) while false do print(x) end return x end
function M.q(x) return x and K, K and x end
function M.r() return math.pi, math.huge end
function M.s(x) return math.sqrt(16), math.floor(2.5), math.abs(-1) + x end
local function outer(x) local function inner(y) return y + 1 end return inner(x) * 2 end
function M.t(x) return outer(x) end
local function big(x)
    local a = x + 1
    local b = a * 2
    local c = b - 3
    local d = c / 4
    local e = d % 5
    local f = e ^ 2
    return f + a + b + c + d + e
end
function M.u(x) return big(x), big(x + 1) end
print(M.a(1), M.b(1, 2), M.c(nil), M.c(1), M.d(3), M.d(2), M.e(-1), M.e(99), M.e(5), M.f(1), M.g(2), M.h("x"))
print(M.i(1)) print(M.j(), M.k(2)) M.l({1, 2}) print(M.m()) print(M.n(1), M.o(2), M.p(3), M.q(1)) print(M.r()) print(M.s(1)) print(M.t(1), M.u(1))
