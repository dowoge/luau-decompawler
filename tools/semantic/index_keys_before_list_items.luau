-- Explicit `[n]` keys land where Luau puts them (array or hash part, counted in
-- the allocation), and later stores grow that allocation the way Luau does.
local function listing(tag, t)
	print(tag, #t)
	for key, value in next, t do
		print(type(key), key, value)
	end
end

local mixed = { [1] = "a", [false] = "b", "c" }
listing("mixed", mixed)
mixed.kind = nil
mixed.slot = "d"
mixed.rank = "e"
mixed.size = "f"
listing("mixed grown", mixed)

local zeros = { [-0] = 1, [1] = 2, [-0] = 3, [3] = nil, "w", "x", "y" }
listing("zeros", zeros)
zeros.kind = "z"
zeros.slot = "zz"
listing("zeros grown", zeros)

local record = { [1] = "p", [2] = "q", name = "r", "s" }
listing("record", record)
record.more = "t"
record.last = "u"
listing("record grown", record)

local sparse = { [1] = nil, [2] = "b", [3] = "c" }
listing("sparse", sparse)
sparse[25] = 25
sparse[3] = nil
sparse[26] = 26
sparse[1] = "a"
listing("sparse later", sparse)

local shuffled = { [2] = "two", [1] = "one", [4] = "four" }
listing("shuffled", shuffled)
shuffled[3] = "three"
listing("shuffled filled", shuffled)
