-- A `while true` whose first block already opens a `for`, and `while` tests built from
-- inlined helpers: each test runs once per trip, after `continue` too; NaN tests stay as written.
local function dayCycle(lamps, windows, hours)
	local hour = 0
	local log = {}
	while true do
		hour += 1
		local light = math.clamp(hour / hours, 0, 1)
		for _, lamp in lamps do
			if lamp.Enabled then
				lamp.Brightness = lamp.Max * (1 - light)
			end
		end
		for _, window in windows do
			if window.Enabled then
				window.Glow = window.Base + (1 - window.Base) * light
			end
		end
		log[#log + 1] = string.format("%.2f", light)
		if light >= 1 then
			return table.concat(log, " "), lamps[1].Brightness, windows[1].Glow
		end
	end
end
print(dayCycle({ { Enabled = true, Max = 3 } }, { { Enabled = true, Base = 0.2 }, { Enabled = false, Base = 1 } }, 4))

local ticks = {}
local function burning(target)
	return target ~= nil and target.Fuel > 0
end
local function burn(target)
	while burning(target) do
		target.Fuel -= 3
		if target.Fuel % 2 == 0 then
			continue
		end
		ticks[#ticks + 1] = target.Fuel
	end
	return target and target.Fuel
end
for _, fuel in { 0, 3, 11 } do
	print("burn", burn({ Fuel = fuel }))
end
print("none", burn(nil))

local asks, served = 0, 0
local function queueEmpty()
	asks += 1
	return served >= 5
end
while true do
	if queueEmpty() then
		break
	end
	served += 1
	if served % 2 == 1 then
		continue
	end
	ticks[#ticks + 1] = -served
end
print("asks", asks, served)

local undefined = 0 / 0
local spins = 0
while true do
	if undefined >= 0 then
		break
	end
	spins += 1
	if spins == 4 then
		break
	end
end
local laps = 0
while true do
	laps += 1
	if laps == 3 then
		break
	end
	ticks[#ticks + 1] = laps * 1000
end
print("nan", spins, "laps", laps)
print(table.concat(ticks, ","))
