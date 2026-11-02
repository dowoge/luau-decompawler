-- A helper copied in takes another call's result as an argument, then reads
-- it in one arm of a choice only. That call ran first, whichever arm runs;
-- inside the arm it would run only with it. (Its `print` shows whether.)
local function nothing(_, _, _)
end
local function step(value, count)
	if 1 == if nothing(1) then count else -1 % 5 - count + math.abs(1) then
		return value
	end
	if count > -count then
		return count * 3 - count * -1 * 3 > count % 3
	end
	local spare = {}
	print(1)
end
local list = {-3}
local box = {x = 8, y = 6}
function box:get() return self.x * 2 + self.y end
local one = 1
local size = math.abs(one)
local rest = (-one) % 5
local total = box:get()
local copy = total
for i, _ in ipairs(if 1 // 3 > box.y then list else {rest - 1 * 0.5 - 0.5}) do
	if "" .. 1 .. `<{size}:{"bc"}>` == 1 then
	else
		box.x = one // 3 < 5 - i % 5 and -copy or copy
	end
end
local result = if step(step(-size == #list, box.x), 1) then 1 % 5 else math.max(1)
print(result)
