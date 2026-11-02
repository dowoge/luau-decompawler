-- A service handle, a computed offset and a guarded field passed into inlined helpers:
-- each argument is computed once, where the source computes it, before the helper runs.
local calls = {}
local services = { Mixer = { id = "mixer" }, Chat = { id = "chat" } }
local game = {}
function game:GetService(name)
	calls[#calls + 1] = "get:" .. name
	return services[name]
end

local function applyGain(bus, db)
	bus.Gain = db
	bus.Peak = db * 1.5
	bus.Trim = bus.Gain - 3
	calls[#calls + 1] = "gain:" .. bus.id
	return bus.Trim
end

local trims = {}
for _, name in ipairs({ "Mixer", "Chat" }) do
	local svc = game:GetService(name)
	trims[#trims + 1] = applyGain(svc, #name * 2)
end
trims[#trims + 1] = applyGain(game:GetService("Chat"), 1)
print(table.concat(trims, " "), table.concat(calls, " "))

local function shelve(box, height)
	box.Height = height
	box.Top = box.Floor + height
	box.Tag = "@" .. height
end

local boxes = { { Floor = 0 }, { Floor = 5 }, { Floor = 9 } }
local level = 2
for i = #boxes, 1, -1 do
	local height = level * i + 1
	shelve(boxes[i], height)
end
print(boxes[1].Top, boxes[2].Tag, boxes[3].Top)
local j = 0
repeat
	j += 1
	shelve(boxes[j], j * level - 1)
until j == #boxes
print(boxes[1].Tag, boxes[2].Top, boxes[3].Height)

local function healthy(stats)
	return stats ~= nil and stats.hp > 0
end

local function rollCall(units)
	local present = {}
	local i = 0
	while i < #units do
		i += 1
		local unit = units[i]
		local stats = unit and unit.stats
		if not healthy(stats) then
			continue
		end
		present[#present + 1] = unit.id
	end
	return table.concat(present, "+")
end

local function leader(units)
	for i = #units, 1, -1 do
		local stats = units[i].stats
		if healthy(stats) and stats.rank > 1 then
			return units[i].id
		end
	end
	return "none"
end

local squad = {
	{ id = "u1", stats = { hp = 5, rank = 3 } },
	{ id = "u2", stats = { hp = 0, rank = 9 } },
	{ id = "u3" },
	{ id = "u4", stats = { hp = 2, rank = 1 } },
}
print(rollCall(squad), leader(squad))
