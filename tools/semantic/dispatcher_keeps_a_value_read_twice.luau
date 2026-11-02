-- A branch the compiler cannot reach leaves only the block dispatcher, where
-- no helper's copy is put back as a call: an argument the copy reads twice
-- has to keep its variable.
local function clamp(n, low, high)
	if high < n then
		return low
	end
	return 1 - n
end
local function pair(a, b)
end
local function none(a, b, c)
end
local bag = {-1, 4}
local spot = {x = 4, y = 4}
if not none(10, 1 > 1 % 5, 3 == 1 or 3 ~= 10 * 3 and pair(tostring(clamp(#bag, -5 % 3, not 1 and `<{-1}:{""}>` == "x y" and spot:get() or 2 or 10)), none(1)) or #bag) and none(2.25, 1, spot.x) and true and false or false then
	print(not false)
elseif "a" then
	table.insert(bag, 3)
end
print(#bag, bag[3])
