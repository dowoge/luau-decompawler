-- Copies of small random helpers several to a line, spread over a multi-line call and
-- nested inside one another: every draw happens once, in order, with its own bounds.
local seed = 77
local function nextRoll()
	seed = (seed * 16807 + 11) % 2147483647
	return seed / 2147483647
end

local function between(lo, hi)
	return lo + (hi - lo) * nextRoll()
end

local function color(r, g, b)
	return string.format("%.4f/%.4f/%.4f", r, g, b)
end

local function lootColor(tint)
	return color(
		between(0.2, tint),
		between(tint, 0.9),
		between(0, 0.1)
	)
end

local function lootStats(level)
	local damage = between(level, level * 2)
	if damage > level * 1.8 then
		print("crit roll", string.format("%.4f", damage))
		damage = math.floor(damage)
	end
	return damage, between(-1, 1)
end

local function chest(level)
	local items = {}
	for i = 1, 2 do
		local dmg, luck = lootStats(level + i)
		local shade = lootColor(0.5)
		items[i] = string.format("%.4f:%.4f:%s", dmg, luck, shade)
	end
	return table.concat(items, " ; "), between(0, 100)
end

print(chest(3))
print(color(between(1, 2), between(3, 4), between(5, 6)), between(-0.5, 0.5))
print(chest(9))
