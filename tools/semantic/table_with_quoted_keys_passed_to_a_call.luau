-- `{ ["k"] = v }` is made with NEWTABLE and a store per key, where `{ k = v }`
-- comes from a template: as an argument it still is one, not a local made
-- before the call
local reads = {}
local probe = setmetatable({}, {
  __index = function(_, k)
    table.insert(reads, k)
    return #reads
  end,
})

local made = {}
local box = {}
function box:Create(a, b, goal)
  return a .. "/" .. b .. "/" .. tostring(goal.Size) .. "/" .. tostring(goal.Name)
end

for i = 1, 2 do
  table.insert(made, box:Create(i, probe.first, { ["Size"] = probe.size * 2, ["Name"] = "n" .. i }))
end
print(made[1], made[2], table.concat(reads, ","))

function box.count(t, extra)
  local n = 0
  for _ in t do
    n += 1
  end
  return n, extra
end
print(box.count({ ["a"] = probe.a, ["b"] = probe.b }, probe.c), table.concat(reads, ","))
print(select("#", box.count({ ["only"] = tostring(7) })))
