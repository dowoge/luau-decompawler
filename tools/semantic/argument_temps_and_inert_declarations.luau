-- Copies of looping helpers whose argument temps are computed inside a larger
-- expression, with the caller's own declarations (a constant, an unset local) between.
local function fold(text, salt)
	local acc = salt
	for i = 1, #text, 2 do
		acc = (acc * 131 + string.byte(text, i)) % 9973
	end
	return acc
end

function weightOf(entry, bias)
	local spread = fold(entry.Key, 17) % 100 / 100
	return bias * math.clamp(spread, 0.25, fold(entry.Key, 3) % 100 / 50 + 0.3), spread
end

function tintOf(key, enabled)
	local hue = fold(key, 1) % 360
	local glow
	local boost = fold(key, 2) % 10 / 10 + 1
	if enabled then
		glow = hue * boost
	end
	return hue, glow, boost
end

print(weightOf({ Key = "oak-plank" }, 4))
print(tintOf("ember", true))
print(tintOf("ember", false))

local shop = { Shelf = { Lamp = "lamp", Rug = "rug" } }
local ledger = {}
local meters = { ticks = 0 }

local function deepGet(cursor, path)
	for part in string.gmatch(path, "[^/]+") do
		cursor = cursor[part]
		if cursor == nil then
			return "?"
		end
	end
	return cursor
end

local function capped(list)
	local s = 0
	for i = 1, #list do
		s += list[i]
		if s > 30 then
			return 30
		end
	end
	return s
end

local function audit(where, budgets)
	local first, second = deepGet(shop, where.first), deepGet(shop, where.second)
	print(first .. "|" .. second, capped(budgets[2]) - capped(budgets[1]))
end

local function tally()
	local n = 0
	for _ in pairs(ledger) do
		n += 1
	end
	return n
end

local function census()
	return { label = "census", entries = tally(), ticks = meters.ticks }
end

audit({ first = "Shelf/Lamp", second = "Shelf/Desk" }, { { 4, 8 }, { 20, 15, 1 } })
audit({ first = "Shelf/Rug", second = "Attic" }, { { 31 }, {} })
local hooks = { census = census }
ledger.alpha = 1
ledger.beta = 2
meters.ticks = 7
local result = hooks.census()
print(result.label, result.entries, result.ticks)
