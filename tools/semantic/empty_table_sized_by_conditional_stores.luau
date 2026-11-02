-- `local slots = {}` gets the array size Luau predicts from the index stores
-- after it, including a store a constant argument later skips.
local function equip(primary, secondary, utility)
	local slots = {}
	if primary then
		slots[1] = primary
	end
	if secondary then
		slots[2] = secondary
	end
	print("utility", utility)
	slots[3] = utility
	return slots
end

local kit = equip("sword", nil, "rope")
print(#kit, kit[1], kit[2], kit[3])
