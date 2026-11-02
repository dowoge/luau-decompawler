-- A key repeated in a constructor still counts toward its allocation, so the
-- first copy cannot be dropped without changing pairs order.
local function dump(t)
	local parts = {}
	for k, v in pairs(t) do
		table.insert(parts, tostring(k) .. "=" .. tostring(v))
	end
	return table.concat(parts, ",")
end

local slot = "Name"
local blank = { [slot] = nil, Position = 2, AnchorPoint = 3, ZIndex = 4, Visible = 5, Image = 6, ImageColor3 = 7, ScaleType = 8, [slot] = 1, "array" }
print(dump(blank))
local seeded = { [slot] = 0, Position = 2, AnchorPoint = 3, ZIndex = 4, Visible = 5, Image = 6, ImageColor3 = 7, ScaleType = 8, [slot] = 1, "array" }
print(dump(seeded))

local key = 0.75
local stats = { [key] = 0, Speed = 2, Armor = 3, Range = 4, [key] = 1 }
print(dump(stats))
stats.Damage = 5
stats.Cooldown = 6
stats[30] = 30
print(dump(stats))

local function spring(initial, damping, cap)
	local self = {
		Position = initial,
		Velocity = if type(initial) == "table" then initial else initial * 0,
		Damping = damping,
		Cap = if cap ~= nil then cap else math.huge,
	}
	return self
end
local a = spring(4, 0.3)
local b = spring({ 2 }, 0.1, 25)
print(a.Position, a.Velocity, a.Damping, a.Cap)
print(b.Velocity[1], b.Damping, b.Cap)
local names = {}
for name in a do
	table.insert(names, name)
end
table.sort(names)
print(table.concat(names, ","))
