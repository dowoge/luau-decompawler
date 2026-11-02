-- A five-field template where one value is nil at runtime, then three new
-- fields: a later key may take the nil node, so the fields stay stores.
local function partFields(t)
	local parts = {}
	for key, value in pairs(t) do
		parts[#parts + 1] = key .. "=" .. tostring(value)
	end
	return table.concat(parts, ",")
end

local function makePart(size, color, material, shape, name)
	local part = { Size = size, Color = color, Material = material, Shape = shape, Name = name }
	part.Anchored = 6
	part.CanCollide = 7
	part.Parent = 8
	return part
end

print(partFields(makePart(1, 2, 3, 4, 5)))
print(partFields(makePart(1, nil, 3, 4, 5)))
print(partFields(makePart(1, 2, nil, 4, 5)))
