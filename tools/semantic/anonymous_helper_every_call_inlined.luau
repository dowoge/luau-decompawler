-- Functions bound with `local name = function` whose every call was inlined: the copies
-- keep working, one of them over a run-time local, another folded to constants.
local checksum = function(n, mix)
	if mix then
		return (n * 31 + n // 3) % 97
	end
	return n
end
local margin = tonumber("5")
local nudge = function(n)
	return math.max(n - margin, 0)
end
local cap = function(n)
	if n < 0 then
		return 0
	end
	return math.min(n, 50)
end

local out = {}
for i, n in ipairs({ 12, 40, 7 }) do
	local c = checksum(n, i ~= 2)
	out[#out + 1] = c .. ":" .. nudge(c)
end
local item = { tier = 3 }
item.tier = nudge(item.tier + 9)
print(table.concat(out, " "), item.tier, cap(-8), cap(120))
