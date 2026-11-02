-- Names the decompiler derives from fields, children, keys and branch values must stay
-- apart wherever the values differ: `_N` spelled by the source, odd plurals, taken names.
local Avatar = {}
function Avatar.accessory(outfit, wantHat)
	local piece
	if wantHat then
		piece = outfit.Hat
	else
		piece = outfit.Cape
	end
	return piece
end
function Avatar.hatOf(mine, theirs, useMine)
	local piece
	if useMine then
		piece = mine.Hat
	else
		piece = theirs.Hat
	end
	return piece
end
function Avatar.hatOr(outfit, spare, useOutfit)
	local piece
	if useOutfit then
		piece = outfit.Hat
	else
		piece = spare
	end
	return piece
end
local outfit = { Hat = "crown", Cape = "red" }
for _, flag in { false, true } do
	print(Avatar.accessory(outfit, flag), Avatar.hatOf(outfit, { Hat = "cap" }, flag), Avatar.hatOr(outfit, "bandana", flag))
end

local function lean(rig)
	local tilt = rig.Tilt
	if tilt < 0 then
		tilt = -tilt
	end
	tilt += 15
	local facing = rig:Facing(3)
	if facing < 0 then
		facing = -facing
	end
	return tilt, facing
end
print(lean({
	Tilt = -10,
	Facing = function(_, n)
		return n - 8
	end,
}))

local INHERIT = {}
local function colorFor(theme, part)
	local color = theme._perPart[part]
	if color == nil then
		color = theme._base
	elseif color == INHERIT then
		color = nil
	end
	return color
end
local theme = setmetatable({ _perPart = { Arm = INHERIT, Leg = "blue" } }, {
	__index = function(_, key)
		print("fallback", key)
		return "grey"
	end,
})
print(colorFor(theme, "Arm"), colorFor(theme, "Leg"), colorFor(theme, "Head"))

local function nameTags(isRed, text)
	local banner = {}
	banner.Name = "Welcome"
	banner.Value = 7
	local team = {}
	if isRed then
		team.Name = "Red"
	else
		team.Name = "Blue"
	end
	local draft = {}
	draft.Name = "Pending"
	draft.Name = text .. "!"
	local found = {}
	found.Name = "Pending"
	local at
	at, found.Name = string.find(text, "e")
	return banner.Name, banner.Value, team.Name, draft.Name, at, found.Name
end
print(nameTags(true, "hello"))
print(nameTags(false, "rover"))

local Gallery = {}
Gallery.__index = Gallery
Gallery.Mice = { "m1", "m2" }
Gallery.Vertices = { 4 }
Gallery.Criteria = { "fast", "cheap" }
Gallery.Feet = { "left" }
for _, v in Gallery.Mice do
	print("mouse", v)
end
for _, v in Gallery.Vertices do
	print("vertex", v)
end
for _, v in Gallery.Criteria do
	print("criterion", v)
end
for _, v in Gallery.Feet do
	print("foot", v)
end
print(Gallery.__index == Gallery)

local Car = {
	FindFirstChild = function(_, name)
		return "<" .. name .. ">"
	end,
}
local function wheels(model)
	local front = model:FindFirstChild("Wheel_1")
	local rear = model:FindFirstChild("Wheel_4")
	return front, rear
end
local function stages(save)
	local early, late, boss = save.zone_1, save.zone_9, save.Boss_100
	save.copy = { early, late, boss }
	return early, late, boss
end
local placed = {}
local Prop = {}
Prop.__index = Prop
function Prop:Clone()
	local label = self.label
	return setmetatable({}, {
		__newindex = function(_, k, v)
			placed[#placed + 1] = label .. ":" .. k .. "=" .. v
		end,
	})
end
local function drop(prop, onLeft)
	if onLeft then
		prop:Clone().Parent = "Left"
	else
		prop:Clone().Parent = "Right"
	end
	prop:Clone().Parent = "Store"
end
print(wheels(Car))
print(stages({ zone_1 = "z1", zone_9 = "z9", Boss_100 = "b100" }))
drop(setmetatable({ label = "Barrel" }, Prop), true)
drop(setmetatable({ label = "Crate" }, Prop), false)
print(table.concat(placed, " "))

local function loadProfile(data)
	local loader = data._loadProfile
	if loader then
		return loader()
	end
	return data._profile
end
local function glow(self, opts)
	local glowing = opts.Glow
	local mine = self._glow
	mine.Size = glowing.Size or opts.Size
	mine.Hue = glowing.Hue
	return mine.Size, mine.Hue
end
local function gap(part, other)
	local a = part.CFrame
	local b = other.CFrame
	return b.X - a.X, a.Z * b.Z
end
print(loadProfile({ _profile = "saved" }), loadProfile({
	_loadProfile = function()
		return "fresh"
	end,
}))
print(glow({ _glow = {} }, { Glow = { Hue = 30 }, Size = 4 }))
print(gap({ CFrame = { X = 1, Z = 2 } }, { CFrame = { X = 6, Z = 5 } }))
