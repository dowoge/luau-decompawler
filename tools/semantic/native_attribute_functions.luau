-- `@native` on a local function, a table function and an anonymous function
-- (but not on the plain one) must not change what any of them computes.
local Mathx = {}

@native local function double(x)
	return x * 2
end

@native function Mathx.shift(x)
	return x - 3
end

local function plain(x)
	return x + 1
end

local names = setmetatable({}, {
	__index = @native function(_, key)
		return "<" .. key .. ">"
	end,
})

local acc = 0
for i = 1, 4 do
	acc += double(i) + Mathx.shift(i) * plain(i)
end
print(acc, names.zap)
