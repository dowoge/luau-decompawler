-- Field stores right after a constructor with `[n]` keys and list items must
-- not join it: that grows its allocation and pairs order changes.
local function listing(tag, t)
	print(tag, #t)
	for key, value in next, t do
		print(type(key), key, value)
	end
end

local mixed = { [1] = "a", [false] = "b", "c" }
mixed.kind = nil
mixed.slot = "d"
mixed.rank = "e"
mixed.size = "f"
listing("mixed", mixed)

local zeros = { [-0] = 1, [1] = 2, [-0] = 3, [3] = nil, "w", "x", "y" }
zeros.kind = "z"
zeros.slot = "zz"
listing("zeros", zeros)
