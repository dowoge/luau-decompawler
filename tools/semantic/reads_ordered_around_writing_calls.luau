-- A stamina value read before a call that changes it (via a closure, a resumed coroutine
-- or an `__index`) keeps the old value; the call may not move across that read.
local stamina = 30
local function current()
	return stamina
end
local function sprint()
	local before = current()
	stamina = 0
	return "sprinted from " .. before
end
print(sprint())

local function regen(rested)
	local pool
	if rested then
		pool = 8
	else
		pool = 3
	end
	local function boost()
		pool = pool + 100
		return pool
	end
	local function look()
		return pool * 2
	end
	local snapshot = pool
	local doubled = look()
	local boosted = boost()
	local after = pool
	return snapshot, doubled, boosted, after
end
print(regen(true))
print(regen(false))

local runner
local reader
runner = coroutine.create(function(fast)
	local speed
	if fast then
		speed = 16
	else
		speed = 8
	end
	reader = function()
		local before = speed
		coroutine.resume(runner)
		return before, speed
	end
	coroutine.yield()
	speed = speed * 2
end)
coroutine.resume(runner, true)
print(reader())

local function settled(fast)
	local speed
	if fast then
		speed = 5
	else
		speed = 2
	end
	local function scaled()
		return speed * 10
	end
	local copy = speed
	return scaled(), copy
end
print(settled(true), settled(false))

local jumps = 0
local Legs = {}
function Legs.hop()
	jumps = jumps + 1
	return 3
end
local beforeHop = jumps
local height = Legs.hop()
print(beforeHop + height, jumps)

local function hopThenRead()
	local h = Legs.hop()
	local j = jumps
	return h * 10 + j
end
print(hopThenRead(), jumps)

local lookups = 0
Stats = setmetatable({}, {
	__index = function()
		lookups = lookups + 1
		return 4
	end,
})
local agility = Stats.Agility
local seenLookups = lookups
local total = agility + 1
print(total, seenLookups, lookups)
