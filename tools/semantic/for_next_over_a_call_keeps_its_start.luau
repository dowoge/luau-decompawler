-- `for k, v in next, f() do`: the call gives both the table and the key that
-- `next` starts after; dropping that key visits the keys before it too
local t = { "a", "b", "c" }
local box = {}
function box.both()
  return t, 1
end
function box:pair()
  return t, 2
end

local seen = {}
for k, v in next, box.both() do
  table.insert(seen, k .. v)
end
for k, v in next, box:pair() do
  table.insert(seen, k .. v)
end

local function count(x)
  local n = 0
  for _, v in next, x:pair() do
    n += #v
  end
  return n
end
print(table.concat(seen, ","), count(box))

-- a call that gives only the table starts at the first key
local all = {}
for _, v in next, table.clone(t) do
  table.insert(all, v)
end
print(table.concat(all, ","))
