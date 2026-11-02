-- A two-field template grown by a later method store rehashes; one three-field
-- template would lay the keys out differently, so pairs order changes.
local function fieldOrder(t)
	local names = {}
	for key in pairs(t) do
		names[#names + 1] = key
	end
	return table.concat(names, ",")
end

local function newNpc(name, health)
	local npc = { Name = name, Health = health }
	npc.Greet = function(self)
		return "hi " .. self.Name
	end
	return npc
end

local guard = newNpc("Guard", 80)
print(fieldOrder(guard), guard:Greet())
