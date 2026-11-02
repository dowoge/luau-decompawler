-- Folded NaN constants keep their sign bit (0/0 and -(0/0) differ); writing
-- them into a buffer shows the high word of each double.
local cell = buffer.create(8)
local function highWord(x)
	buffer.writef64(cell, 0, x)
	return buffer.readu32(cell, 4)
end

print(highWord(0 / 0), highWord(-(0 / 0)))
print(highWord(math.huge - math.huge), highWord(-(math.huge - math.huge)))
local base = 3
print(highWord(base + -(0 / 0)), highWord(-(-(0 / 0))))
