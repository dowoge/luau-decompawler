-- Luau sizes `local t = {}` (and `local t = setmetatable({}, mt)`) for the
-- `t[1], t[2] ...` and `t.name` stores that follow; a table it did not size
-- keeps a different array part, which `#` with a hole shows
local function holes(t)
  t[1] = "a"
  t[2] = nil
  t[3] = "c"
  return #t
end

local grouped = ({})
grouped[1] = 1 grouped[2] = nil grouped[3] = 3
local sized = {}
sized[1] = 1 sized[2] = nil sized[3] = 3
print(#grouped, #sized)

local p, q = {}, {}
p[1] = true p[2] = nil p[3] = true
q.name = "q"
print(#p, q.name)

local mt = {__index = function(_, k) return "meta " .. tostring(k) end}
local plain = setmetatable({}, mt)
plain[1] = 1 plain[2] = nil plain[3] = 3
local wrapped = setmetatable(({}), mt)
wrapped[1] = 1 wrapped[2] = nil wrapped[3] = 3
print(#plain, #wrapped, wrapped[2])

local looped = ({})
for i = 1, 5 do
  if i % 2 == 1 then looped[i] = i end
end
local counted = {}
for i = 1, 5 do
  if i % 2 == 1 then counted[i] = i end
end
print(#looped, #counted, holes({}), holes(({})))

local named = ({})
named.x = 1 named[1] = "one" named[2] = nil named[3] = "three"
print(#named, named.x)
