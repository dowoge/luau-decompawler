-- `for _ in step, bag.Tools` with `step` a local alias of `next`: the state
-- expression must survive, or the loop iterates nothing (or errors).
local step = next
local walk = ipairs
local Inventory = {}

function Inventory.weigh(bag)
	local weight = 0
	for _, tool in walk(bag.Tools) do
		weight += tool.Mass
	end
	local count = 0
	for _ in step, bag.Tools do
		count += 1
	end
	local slots = 0
	for _, _ in step, bag.Slots do
		slots += 1
	end
	return weight, count, slots
end

print(Inventory.weigh({ Tools = { { Mass = 3 }, { Mass = 5 } }, Slots = { "a", "b", "c" } }))
