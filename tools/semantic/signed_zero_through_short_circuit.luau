-- `x and y` yields y itself, so zeros of opposite sign stay distinct (as 1/x
-- shows), whether operands are runtime values, locals or folded constants.
local Pick = {}
function Pick.second(a, b)
	return 1 / (a and b)
end
function Pick.first(a, b)
	return 1 / (a or b)
end
print(Pick.second(-0, 0), Pick.second(0, -0), Pick.first(-0, 0), Pick.first(0, -0))

local function inlined(a, b)
	return 1 / (a and b)
end
print(inlined(-0, 0), inlined(0, -0))

local neg, pos = -0, 0
print(1 / (neg and pos), 1 / (pos and neg), 1 / (neg or pos))

local z = 0
local nz = -z
print(1 / (z and nz), 1 / (nz and z), 1 / (nz or 5))
