-- One function literal made again (a loop Luau unrolls at -O2) is one
-- closure while what it captures is the same: written out once per copy it
-- would be several functions
local m = tonumber("8")
local p = tonumber("2")
local function helper() return m end

local hs = {}
for i = 1, 2 do hs[i] = function() return helper() end end
print("unrolled", hs[1] == hs[2], hs[1]())

local function inner()
  local fs = {}
  for i = 1, 3 do fs[i] = function() return m, p end end
  return fs[1] == fs[2], fs[2] == fs[3]
end
print("unrolled inside", inner())

local seen = {}
local function keep(f) seen[#seen + 1] = f return f end
for i = 1, 3 do keep(function() return p * 2 end) end
print("passed on", seen[1] == seen[2], seen[2] == seen[3], seen[3]())

-- each round's own local: a closure each
local loose = {}
for i = 1, 2 do
  local c = tonumber("3")
  loose[i] = function() return c end
end
print("own local each round", loose[1] == loose[2])
