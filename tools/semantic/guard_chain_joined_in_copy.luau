-- Guard chains copied into callers, alone and nested inside another helper's copy:
-- every test still runs in order and gives the helper's own answer.
local events = {}
local function queueRun(fn)
	events[#events + 1] = "queueRun"
	fn()
end

local Launcher = {}
Launcher.__index = Launcher

function Launcher:Fire(shot, rules, pattern, owner)
	local owned = owner ~= nil
	local round = self.round or 0
	local function inFlight()
		if not self.Armed or (self.round or 0) ~= round or shot.Hit or owned and shot.Owner ~= owner then
			return false
		end
		return not (rules and rules.ammo and rules.ammo[1] == 0)
	end
	local function travel(steps)
		for _ = 1, steps do
			if not inFlight() then
				events[#events + 1] = "stop"
				return
			end
			events[#events + 1] = "move"
			if shot.Bouncy then
				queueRun(function()
					if inFlight() then
						events[#events + 1] = "bounce"
					end
				end)
			end
		end
	end
	if pattern == "burst" then
		travel(2)
	else
		queueRun(function()
			if not inFlight() then
				return
			end
			events[#events + 1] = "single"
		end)
	end
	return #events
end

local launcher = setmetatable({ Armed = true }, Launcher)
print(launcher:Fire({ Bouncy = true }, nil, "burst", nil))
print(launcher:Fire({ Owner = 1 }, { ammo = { 0 } }, "single", 1))
print(launcher:Fire({ Owner = 2 }, { ammo = { 3 } }, "single", 1))
print(launcher:Fire({ Hit = true }, nil, "burst", nil))
launcher.round = 5
print(launcher:Fire({}, { ammo = {} }, "single", nil))
print(table.concat(events, ","))

function canonical(value)
	if type(value) == "number" then
		return string.format("#%03d", value)
	end
	if type(value) ~= "string" or value == "" then
		return nil
	end
	return value
end

local function insertOnce(set, key)
	if key == nil or set.has[key] then
		return false
	end
	set.has[key] = true
	set.order[#set.order + 1] = key
	return true
end

local function addId(set, value)
	local key = canonical(value)
	if key == nil then
		return false
	end
	return insertOnce(set, key)
end

function collectIds(bonus)
	local set = { has = {}, order = {} }
	local results = {
		addId(set, 7),
		addId(set, "#007"),
		addId(set, "door"),
		addId(set, bonus),
	}
	return #set.order, table.concat(set.order, " "), tostring(results[2]), tostring(results[4])
end
print(collectIds(""))
print(collectIds(12))
print(collectIds("door"))
