-- A local set by if/else (one arm calls out) is computed before the expression
-- using it; folded into a select there, effects and lookups would reorder.
local loads = 0
local function load()
	loads += 1
	return "asset"
end
local function maybeAsset(cached, allowed)
	local value
	if cached then
		value = false
	else
		value = load()
	end
	return allowed and value
end
print(maybeAsset(false, false), loads)
print(maybeAsset(false, true), loads)
print(maybeAsset(true, true), loads)

local phase = "old"
local oldBin, newBin = {}, {}
local router = setmetatable({}, {
	__index = function()
		return if phase == "old" then oldBin else newBin
	end,
})
local function advance()
	phase = "new"
	return "item"
end
local function file(skip)
	local value
	if skip then
		value = false
	else
		value = advance()
	end
	router.bin[1] = value
end
file(false)
print(oldBin[1], newBin[1])

local handler = function(v)
	return "first handler", v
end
local function rewire()
	handler = function(v)
		return "second handler", v
	end
	return 7
end
local function dispatch(skip)
	local value
	if skip then
		value = false
	else
		value = rewire()
	end
	return handler(value)
end
print(dispatch(false))

local offset = 1000
local function reset()
	offset = 1
	return 5
end
local function shifted(skip)
	local value
	if skip then
		value = 0
	else
		value = reset()
	end
	return offset + value
end
print(shifted(false))

local made = 0
local function craft()
	made += 1
	return "plank"
end
local function bundle(cached, key)
	local value
	if cached then
		value = false
	else
		value = craft()
	end
	return { [key] = "tag", value }
end
print((pcall(bundle, false, nil)), made)
print((pcall(bundle, false, 0 / 0)), made)
local ok, t = pcall(bundle, false, "label")
print(ok, t.label, t[1], made)
