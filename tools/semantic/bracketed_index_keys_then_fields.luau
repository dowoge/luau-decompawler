-- Bracketed index keys share the constructor's allocation with its fields;
-- the field stores after it grow the table later, which pairs order shows.
local function dump(t)
	local parts = {}
	for key, value in pairs(t) do
		parts[#parts + 1] = tostring(key) .. "=" .. tostring(value)
	end
	return table.concat(parts, ",")
end

local hotbar = { [1] = nil, owner = "Builder", [2] = "Hammer", mode = "build", [3] = "Saw" }
print(dump(hotbar))
hotbar.selected = 2
hotbar.locked = false
hotbar.theme = "dark"
hotbar[12] = "Spare"
print(dump(hotbar))
