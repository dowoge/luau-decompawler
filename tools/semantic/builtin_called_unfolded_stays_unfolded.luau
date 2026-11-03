-- A builtin called on constants by a spelling Luau does not see as the
-- builtin (`(math.abs)(-17)`) is a real call at run time; spelled plainly
-- it would fold to its value, or be fast-called past a replaced builtin.
local scale = tonumber("2.5")
local a = (math.abs)(-17)
local b = math.abs(-17)
local c = (math.max)(3, 9, 4)
local d = (string.len)("hello")
local e = (tostring)(5)
print(a, b, c, d, e, math.floor(scale))
return a, c
