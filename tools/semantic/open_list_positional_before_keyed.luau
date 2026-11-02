-- A list item before keyed entries and a branch: the open tail after the split
-- continues the numbering and is still stored in one piece.
local trail = {}
local function step(label, value)
	trail[#trail + 1] = label
	return value
end

local function rest(...)
	step("rest")
	return ...
end

local function lineup(slot, flag, ...)
	local function swap()
		slot = "moved"
		return step("swap", "S")
	end
	return {
		step("head", "H"),
		[slot] = "keyed",
		owner = nil,
		if flag then swap() else step("keep", "K"),
		rest(...),
	}
end

local t = lineup("seat", true, "x", nil, "z")
t[8] = "end"
print(#t, table.concat(trail, ","), t.seat, t.moved)
for key, value in next, t do
	print(type(key), key, value)
end
