-- Defaults written with `or`, a typeof test and `and`/`or`, folded where a copy gets a
-- constant or no argument at all: each copy must use the value the call would.
function newOutline()
	return { Kind = "Outline" }
end

local function outline(target, width)
	local o = newOutline()
	o.Width = (width or 1.5) * 4
	o.Target = target.Name
	return o
end

local widths = {}
local card = { Name = "Card" }
for _, o in ipairs({ outline(card, 2), outline(card), outline(card, false), outline(card, nil), outline(card, 0) }) do
	widths[#widths + 1] = o.Width .. o.Target
end
outline(card, 9)
print(table.concat(widths, " "))

local sounds = { ui = { click = "rbxassetid://80", hover = "" } }
local function soundFor(bankName, cue, fallbackId)
	local chosen = "silent"
	if typeof(fallbackId) == "string" then
		chosen = fallbackId
	end
	local bank = sounds[bankName]
	local id = bank and bank[cue]
	if typeof(id) ~= "string" or id == "" then
		return chosen
	end
	return id
end

local fx = {
	Click = soundFor("ui", "click"),
	Hover = soundFor("ui", "hover"),
	Absent = soundFor("game", "boom"),
	Custom = soundFor("game", "boom", "rbxassetid://12"),
	Wrong = soundFor("ui", "hover", 5),
}
print(fx.Click, fx.Hover, fx.Absent, fx.Custom, fx.Wrong)

local trail = {}
local function glow(light, on)
	local aim = { Range = on and 16 or 4 }
	light.Aim = aim
	trail[#trail + 1] = light.Name .. ":" .. aim.Range
	return aim.Range
end

local world = { Name = "World" }
local function spawnPart(class, title, holder)
	local part = { Class = class, Name = title, Parent = holder or world }
	trail[#trail + 1] = title .. "@" .. part.Parent.Name
	return part
end

local function lightsOn(room)
	for _, l in ipairs(room) do
		glow(l, true)
	end
end

local function lightsOff(room)
	local total = 0
	for _, l in ipairs(room) do
		total += glow(l, false)
	end
	return total
end

local function assemble()
	local base = spawnPart("Model", "Base")
	local roof = spawnPart("Part", "Roof", base)
	spawnPart("Part", "Door")
	return base.Parent == world, roof.Parent.Name
end

local room = { { Name = "lamp" }, { Name = "candle" } }
lightsOn(room)
print(lightsOff(room), room[1].Aim.Range)
print(assemble())
print(table.concat(trail, ", "))
