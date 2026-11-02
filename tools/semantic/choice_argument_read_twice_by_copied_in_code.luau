-- A copied-in helper reads its argument twice, and that argument is an
-- `a and b or c` value computed inside an if-expression: it needs a local,
-- which no expression can declare. (The helper's last test folds away for a
-- constant argument, so its copy no longer reads as a call to it.)
local crate = {w = 2, h = 5}
function crate:area()
	return self.w * self.h
end
local function flip(n, floor)
	if (if not (n > floor) then "low" else "high") then
		return -n
	end
	if 1 > 2 - floor then
		print("never")
	end
end
local word = tostring(123)
local scaled = if string.len(word) > 1 then flip(not (1 <= crate:area()) and #crate.items or -1, 0) * 3 else nil
local wide = if string.len(word) > 1 then flip(not (100 <= crate:area()) and crate.w or -1, 0) * 3 else nil
print(scaled, wide)
