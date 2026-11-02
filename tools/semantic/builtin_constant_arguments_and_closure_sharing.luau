-- A builtin given a value that is the same constant on every path, captured or
-- not; closures over true constants may be reused, the others may not.
local Damage = {}
function Damage.base(crit)
	local amount = -40
	if crit then
		amount = -40
	end
	return (math.abs(amount))
end
function Damage.deferred(crit)
	local amount = -40
	if crit then
		amount = -40
	end
	local calc = function()
		return (math.abs(amount)) * 2
	end
	print("amount", amount)
	return calc()
end
function Damage.picked(crit)
	local low = crit and 6 or 6
	local high = crit and 9 or 9
	return (math.clamp(low * 2, low, high))
end
for _, name in { "base", "deferred", "picked" } do
	print(name, Damage[name](true), Damage[name](false))
end

local Factory = {}
function Factory.fixed()
	local radius = 5
	return function()
		return (math.floor(radius * 3.3))
	end
end
function Factory.chosen(flag)
	local radius = flag and 5 or 5
	return function()
		return (math.floor(radius * 3.3))
	end
end
for _, name in { "fixed", "chosen" } do
	local a, b = Factory[name](true), Factory[name](true)
	print(name, a == b, a(), b())
end
