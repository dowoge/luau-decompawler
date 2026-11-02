-- A register operand, builtin argument, method receiver or constructor key is
-- read when its operation runs, after a callback; copies taken first stay old.
local function relay(fn)
	return fn()
end

local function ownerSum(start, later)
	local level = start
	local function bump()
		level = later
		return 1
	end
	return level + relay(bump)
end

local function innerOr(start, later)
	local level = start
	local function bump()
		level = later
		return 1
	end
	local function read()
		return (level or 0) + relay(bump)
	end
	return read()
end

local function innerAnd(start, later)
	local level = start
	local function bump()
		level = later
		return 1
	end
	local function read()
		return (level ~= nil and level) * relay(bump)
	end
	return read()
end

local function copied(start, later)
	local level = start
	local function bump()
		level = later
		return 5
	end
	local before = level
	local less = bump() < before
	local diff = before - bump()
	if bump() > before then
		return less, diff, "grew", level
	end
	return less, diff, "same", level
end

print("owner", pcall(ownerSum, 10, 20))
print("or", pcall(innerOr, 10, 20))
print("and", pcall(innerAnd, 10, 20))
print("copied", copied(1, 10))

local function aliasMax(start, later)
	local score = start
	local best = math.max
	local function bump()
		score = later
		return 3
	end
	local kept = score
	local top = best(kept, (relay(bump)))
	return top, best(score, 0)
end
print("alias", aliasMax(4, 9))

local tally = 0
local Meter = {}
function Meter.tick()
	tally = tally + 100
	return 7
end
local held = tally
local now = Meter.tick()
print("meter", math.min(held, now, 50), tally)

local Red = { color = "red" }
function Red:paint(n)
	return self.color .. n
end
local Blue = { color = "blue", paint = Red.paint }
local function receiver()
	local brush = Red
	local function swap()
		brush = Blue
		return 2
	end
	local kept = brush
	return kept:paint(swap()), brush:paint(1)
end
print("receiver", receiver())

local function keyed()
	local slot = "head"
	local function move()
		slot = "feet"
		return "boots"
	end
	local kept = slot
	local gear = { [kept] = move() }
	return gear.head, gear.feet, slot
end
print("key", keyed())
