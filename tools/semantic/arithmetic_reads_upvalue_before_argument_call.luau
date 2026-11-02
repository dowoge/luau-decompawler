-- `armor - shatter() * 3 + 1` fetches `armor` before `shatter()` raises it;
-- written as `mitigate(shatter())` the call would run first. (pcall keeps `hitCaller` out of line.)
local armor = 2

local function shatter()
	armor = 10
	return 4
end

local function mitigate(dmg)
	return armor - dmg * 3 + 1
end

local function hitCaller()
	return armor - shatter() * 3 + 1, "hit"
end

print(pcall(hitCaller))
print(armor)
return mitigate
