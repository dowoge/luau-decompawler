-- Luau makes one closure for a function literal whose upvalues are top-level
-- locals nothing writes (DUPCLOSURE, reused while the captured values are the
-- same); a local written anywhere is captured by reference, a new closure
-- each time. `==` between two closures shows which
local function same(make)
  return make() == make()
end

-- written again after it is declared: by reference
local p, q = tonumber("1"), tonumber("2")
p, q = q, p
print("swapped", same(function() return function() return p - q end end))

local t = {a = tostring(1), b = "2"}
local x = t.a .. "z"
x = t.b
print("field", same(function() return function() return x end end))

local k = tostring(3)
k = tostring(4)
print("call", same(function() return function() return k end end))

local y = tonumber("5")
local set = function(z) y = z end
set(6)
print("written by a closure", same(function() return function() return y end end))

local function param(v)
  v = tostring(v)
  return same(function() return function() return v end end), v
end
print("parameter", param(7))

-- written nowhere: one closure
local m = tonumber("8")
print("never written", same(function() return function() return m end end))
