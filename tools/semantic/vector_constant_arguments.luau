-- Vector arguments built from constants and folded inside copies: every component,
-- negative zero and float rounding included, survives, and a shadowed library stays live.
local function project(v, axis)
	local d = v.X * axis.X + v.Y * axis.Y + v.Z * axis.Z
	return axis * d
end
local function halve(v)
	return v / 2
end
local function components(v)
	return project(v, Vector3.new(1, 0, 0)), project(v, vector.create(0, 0, -1))
end
for _, v in ipairs({ Vector3.new(4, -2, 6), vector.create(-0.5, 3, 8) }) do
	print("parts", components(v))
end
print("halves", halve(Vector3.new(9, -4, 2)), halve(vector.create(-0, 7, -3)))
print("rounding", halve(Vector3.new(16777217, 0.7, -0)))
local h = halve(vector.create(-0, 0, 5))
print("zeros", 1 / h.X, 1 / h.Y, h.Z)
local realCreate = vector.create
local intercepted = 0
local shim = {
	create = function(x, y, z)
		intercepted += 1
		return realCreate(x * 10, y, z)
	end,
}
local function viaLocal(vector)
	return halve(vector.create(2, 4, 6))
end
print("shadowed", viaLocal(shim), intercepted)
