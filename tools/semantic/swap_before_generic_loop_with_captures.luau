-- `a, b = b, a` on captured locals right before a generic `for`: the swap runs before the
-- loop is prepared, so an `__iter` metamethod already sees the swapped values.
local function once(v)
	local used = false
	return function()
		if used then
			return nil
		end
		used = true
		return v
	end
end

local function alternate(x, y)
	local a, b = x, y
	for lap = 1, 3 do
		a, b = b, a
		for got in once(a .. lap) do
			print(lap, a, got)
		end
	end
	return function()
		return a, b
	end
end
print(alternate("L", "R")())

local function overList(flip, list)
	local a, b = 2, if flip then 3 else -3
	local lap = 0
	repeat
		lap += 1
		a, b = b, a
		for i, v in ipairs(list) do
			print(i, v, a, b)
		end
	until lap >= 2
	local function best()
		return math.max(if flip then a else b, 0)
	end
	return best()
end
print(overList(true, { 7 }), overList(false, {}))

local peek
local watched = setmetatable({}, {
	__iter = function()
		print("iter", peek())
		return next, {}
	end,
})
local function cycle(p, q)
	local a, b = p, q
	peek = function()
		return a
	end
	local laps = 0
	repeat
		laps += 1
		a, b = b, a
		for _ in watched do
		end
	until laps >= 2 or not a
	return a, b, laps
end
print(cycle("x", "y"))
print(cycle("x", false))
