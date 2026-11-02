-- A folded vector constant keeps each component's bits, so a NaN component
-- keeps its sign; float32 bit patterns show it.
local function corners()
	return vector.create(0 / 0, 0, 1), vector.create(-(0 / 0), 0, 1), vector.create(1, -(0 / 0), 0 / 0)
end

local scratch = buffer.create(4)
local function bits(f)
	buffer.writef32(scratch, 0, f)
	return buffer.readu32(scratch, 0)
end

local a, b, c = corners()
print(bits(a.x), bits(b.x), bits(c.y), bits(c.z))
