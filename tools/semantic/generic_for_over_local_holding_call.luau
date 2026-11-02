-- `local nextWave = spawner()` keeps one result, and so does `(spawner())`; neither may
-- become `for wave in spawner()`, which would pass the second result as the loop state.
local function waveAfter(plan, wave)
	wave = (wave or 0) + 1
	local cap = plan and plan.cap or 2
	if wave <= cap then
		return wave
	end
	return nil
end

local function spawner()
	return waveAfter, { cap = 4 }
end

local seen = {}
local nextWave = spawner()
for wave in nextWave do
	seen[#seen + 1] = wave
end
print(#seen, table.concat(seen, " "))

local sum = 0
for wave in (spawner()) do
	sum += wave
end
print(sum)
