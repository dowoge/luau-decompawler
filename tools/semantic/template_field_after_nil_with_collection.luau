-- A call-valued field that churns garbage sits between a nil field and constant
-- ones: the template must keep its keys in source order, or pairs order changes.
local scratch
local function loadWeapon(name)
	for i = 1, 12000 do
		scratch = { i, name }
	end
	return name
end

local function listFields(t)
	local parts = {}
	for key, value in pairs(t) do
		parts[#parts + 1] = key .. ":" .. tostring(value)
	end
	return table.concat(parts, " ")
end

local function spawnState(hostile)
	local state = { Health = 100, Armor = 0, Target = nil, Weapon = loadWeapon("Pike"), Speed = 16, Jump = 50, Level = 1, Team = 0 }
	if hostile then
		state.Target = "player"
	else
		state.Target = "none"
	end
	state.Speed = 20
	return listFields(state)
end

print(spawnState(true))
print(spawnState(false))
