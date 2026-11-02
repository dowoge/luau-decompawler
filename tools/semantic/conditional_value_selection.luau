-- Branches that only choose a value, and `c and x or d`: false and nil stay distinct,
-- result counts hold, a test with effects runs once, and a shadowing local stays scoped.
local Settings = {}
function Settings.option(enabled, onValue, offValue)
	if enabled then
		return onValue
	else
		return offValue
	end
end
local function presets()
	return "low", nil, "ultra"
end
local function maybePresets(enabled)
	if enabled then
		return presets()
	else
		return nil
	end
end
local function asToggle(v)
	if v then
		return true
	end
	return false
end
local option = Settings.option
for _, enabled in { false, true, 0, "", {} } do
	local toggle = asToggle(enabled)
	local shadows
	if enabled then
		shadows = "soft"
	else
		shadows = "hard"
	end
	print(toggle, type(toggle), option(enabled, false, nil), option(enabled, "on", "off"), shadows)
	print("presets", select("#", maybePresets(enabled)), maybePresets(enabled))
end

local quality = "outer"
do
	local quality
	if quality then
		quality = "wrong"
	else
		quality = "inner"
	end
	print("scope", quality)
end
print("scope", quality)

local asked = 0
local function prompt()
	asked += 1
	return asked % 3 == 1
end
local function choice()
	local answer
	if prompt() then
		answer = false
	else
		answer = nil
	end
	return answer
end
print("asked", choice(), choice(), choice(), asked)

local function pickOr(c, x, d)
	return c and x or d
end
print(pickOr(true, 0, 1), pickOr(true, false, 1), pickOr(false, 0, 1), pickOr(1, nil, "fallback"))
local fetches = 0
local remote = setmetatable({}, {
	__index = function(_, key)
		fetches += 1
		return key == "ping" and fetches * 7 or nil
	end,
})
local function pingOr(connected)
	return connected and remote.ping or "offline"
end
print(pingOr(true), pingOr(nil), pingOr(true), fetches)

local function selection(enabled, picked, other)
	local value
	if enabled then
		value = picked
	else
		value = other
	end
	print(value, picked, other)
	return { SelectionImageObjectTransparencyWhenHovered = value }
end
for _, args in { { true, false, 0.25 }, { false, false, 0.25 }, { true, nil, 0.25 } } do
	print(selection(args[1], args[2], args[3]).SelectionImageObjectTransparencyWhenHovered)
end
return selection
