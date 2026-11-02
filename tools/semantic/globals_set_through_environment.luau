-- Globals this script never assigns by name (set through its environment) are
-- looked up where they stand, and an alias of one's field keeps what it read.
local env = _G.getfenv(1)
env.Config = { volume = 2 }
env.setMode = function(m)
	env.Mode = m
end

local function boost()
	Config.volume = Config.volume * 5
	return 6
end
local function verdict(limit)
	if Config.volume > limit then
		return "loud"
	end
	return "soft"
end
local result = verdict(boost())
print(result, Config.volume)

local function apply(m)
	setMode(m)
	if m then
		return "on"
	end
	return "off"
end
for _, m in ipairs({ "night", false }) do
	local label = apply(m)
	print(label, Mode)
	if apply(m) == "on" then
		print("lit", Mode)
	end
end

env.Registry = {
	handler = function()
		return "initial"
	end,
}
local held = Registry.handler
Registry.handler = function()
	return "replaced"
end
print(held())
local second = Registry.handler
local function rotate()
	Registry.handler = function()
		return "rotated"
	end
end
rotate()
print(second(), Registry.handler())
