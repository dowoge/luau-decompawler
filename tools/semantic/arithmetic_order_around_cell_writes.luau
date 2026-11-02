-- Arithmetic that may raise or run a metamethod stays where it is relative to
-- calls writing captured locals, and operands are read when the operator runs.
local stamina = 50
local function drain(amount)
	stamina = stamina - amount
end

local Scale = setmetatable({}, {
	__mul = function(_, n)
		print("scale sees", stamina)
		return n * 2
	end,
})

local function strike(factor)
	local dealt = Scale * factor
	drain(10)
	return dealt
end
print(strike(3), stamina)

local ok = pcall(function()
	local cost = MissingRate * 4
	drain(1000)
	print("unreachable", cost)
end)
print(ok, stamina)

local bias = 2
local Odd = setmetatable({}, {
	__mul = function(_, n)
		bias = 50
		return n
	end,
})
local function weigh(v)
	local step = v * 3 - 1
	return bias * step + 4
end
print(weigh(5), bias)
bias = 2
print(weigh(Odd), bias)
