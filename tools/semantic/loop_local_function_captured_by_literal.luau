-- A literal in a loop that captures only a local function the body declares stays one
-- shared closure; body locals captured by value, or reassigned later, belong to their trip.
local spawners = {}
local wave = 0
while wave < 2 do
	wave += 1
	local function makeZombie()
		return "zombie"
	end
	spawners[wave] = function()
		return makeZombie
	end
end
print(spawners[1] == spawners[2], spawners[2]()())

local doors = {}
for i = 1, 3 do
	local code = "door" .. i * 11
	doors[i] = function()
		return code
	end
end
print(doors[1](), doors[3](), doors[1] == doors[2])

local shields = {}
for i = 1, 2 do
	local hp = i * 40
	shields[i] = function()
		return hp
	end
	hp = hp // 2
end
print(shields[1](), shields[2]())
