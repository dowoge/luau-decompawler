-- Swaps and copies right after a loop that breaks, or around a generic `for`, belong after
-- the loop's exits join; rotations through several locals must not lose a value.
local Lane = {}
function Lane.merge(x, y, stop)
	local a, b, c = x, y, x * 10
	if b then
		for _ = 1, 3 do
			if stop then
				break
			end
		end
		a, c = c, a
	end
	return a, b, c
end
print(Lane.merge(1, 2, true), Lane.merge(1, 2, false))
print(Lane.merge(1, nil, true))

local checks = 0
local function green()
	checks += 1
	return checks % 3 == 0
end
function Lane.waitLight(stop)
	for _ = 1, 4 do
		if stop then
			break
		end
	end
	local lights = 0
	while lights < 2 do
		if green() then
			break
		end
	end
	return checks
end
print(Lane.waitLight(true), Lane.waitLight(false))

local fuel = 0
local function burn()
	fuel += 1
	return fuel < 5
end
function Lane.drift(seed, left, cones)
	local a, b = seed, if left then 3 else -3
	while burn() do
		a, b = b, a
		if a < seed then
			a -= 2
		end
		for _ in cones do
			local wobble = 0
			repeat
				wobble += 1
			until wobble >= 2
		end
	end
	return a, b
end
print(Lane.drift(6, true, { 1 }))
fuel = 0
print(Lane.drift(6, false, {}))

local function pell(n)
	local a, b = 0, 1
	for _ = 1, n do
		a, b = b, 2 * b + a
	end
	return a
end
local function spin(t)
	local x, y, z = t.r, t.g, t.b
	for _ = 1, 5 do
		x, y, z = y, z, x
	end
	return x, y, z
end
print(pell(0), pell(1), pell(6), spin({ r = "red", g = "green", b = "blue" }))
