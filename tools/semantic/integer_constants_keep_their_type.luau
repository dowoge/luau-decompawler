-- Integer constants (`5i`) are a type of their own: written back without
-- the suffix they would come back as numbers, of another type, and past
-- 2^53 with another value.
local small = 5i
local big = 9007199254740993i
local function describe(x)
	return typeof(x), tostring(x)
end
print(describe(small))
print(describe(big))
print(describe(-42i))
local t = { small, big, [7i] = "seven" }
print(#t, t[7i], typeof(t[1]), t[7])
print(small == 5i, rawequal(big, 9007199254740993i))
local n = tonumber("3")
print(typeof(n), n == 3)
return small, big
