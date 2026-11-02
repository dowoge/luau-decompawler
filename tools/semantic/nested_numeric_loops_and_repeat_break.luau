-- Numeric loops nested and accumulating, and a repeat with a guarded break inside a
-- one-pass numeric loop behind odd conditions: every count and checksum must match.
local function odds()
	local rows = {}
	for r = 1, 255, 2 do
		local acc = r
		for c = 3, 1, -1 do
			acc *= c
		end
		rows[(r + 1) // 2] = acc - r
	end
	return rows
end
local rows = odds()
print(#rows, rows[1], rows[2], rows[64], rows[128])

local sum = 0
local budget = 120
local laps = 0
local function mix(value, kind)
	if kind == 1 then
		sum = (sum + value) % 1000003
	elseif kind == 2 then
		sum = (sum + (value and 3 or 7)) % 1000003
	end
	return value
end

if not ({} and (not (mix(("abc"):len(), 1) or bit32.bor(mix(-5, 1))))) then
	marker = 9
end
if (function()
	budget -= 1
	return budget >= 0 and sum < 1e9
end)() then
	for lap = 2, 2 do
		local guard = 0
		repeat
			guard += 1
			laps += 1
			if guard == 4 then
				break
			end
			mix(lap + guard, 1)
		until guard * lap > 6 and {}
		mix(guard, 2)
		break
	end
end
if mix({}, 2) then
	mix(laps, 1)
end
print("checksum", sum, laps, marker, budget)
