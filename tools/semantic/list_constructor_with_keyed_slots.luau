-- A constructor whose keyed entries name slots its list items may also fill: a trailing call
-- returning nothing leaves the keyed value in place, one returning values overwrites it.
local function inventory(owner, spare)
	local function restock(n)
		print("restock", n)
		return table.unpack({ spare, "potion", "elixir" }, 1, n)
	end
	local bag = { [3] = owner .. "'s map", (restock(1) or "empty"), (restock(1)), restock(0) }
	spare = nil
	return bag[1], bag[2], bag[3]
end
print(inventory("Ava", "rope"))

local function overwrite(owner, spare)
	local function restock(n)
		print("restock", n)
		return table.unpack({ spare, "potion", "elixir" }, 1, n)
	end
	local bag = { [1] = owner, [2] = owner, [4] = owner .. "!", (restock(1) or spare), restock(3) }
	spare = nil
	return bag[1], bag[2], bag[3], bag[4]
end
print(overwrite("Ava", "rope"))
