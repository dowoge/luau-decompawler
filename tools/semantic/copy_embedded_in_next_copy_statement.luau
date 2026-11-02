-- Two copies of a hue-wrapping helper in one function, the second fed by the first's
-- value through the statement around it: both wrap exactly as the calls would.
local function clampHue(h)
	h = h % 360
	if h >= 180 then
		h -= 360
	end
	return h
end

local function midHue(a, b)
	local delta = clampHue(b - a)
	return clampHue(a + delta * 0.5)
end

local function chase(current, goal, rate)
	local gap = clampHue(goal - current)
	return clampHue(current + math.clamp(gap, -rate, rate)), gap
end

print(midHue(10, 350), midHue(170, -170), clampHue(725), clampHue(-725))
print(chase(170, -170, 15))
print(chase(-20, 40, 100))
