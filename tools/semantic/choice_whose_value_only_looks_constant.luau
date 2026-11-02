-- A helper copied in as a condition returns `1 .. s == t` on one path and a
-- constant on the other. The comparison only starts like a literal and may be
-- false: written `c and x or y`, the condition would hold when it is false.
-- (`never` is unset: calling it fails.)
local function check(_, fallback, size)
	if not (false or size > size) then
		return 1 .. string.rep("{q}", 2) .. tostring(1) == "{q}"
	end
	return fallback or if size <= size then if fallback then "x y" else "x y" else "x y" == tostring(1)
end
local box = {x = 6, y = 5}
function box:get() return self.x * 2 + self.y end
local result = if check(unset == nil, 1, box.x) then never(1) else nil
print(result)
