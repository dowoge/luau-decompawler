-- Bindings that would earn the same name stay distinct; loop counters and curried values
-- keep their own; updates in place (`x += 1`, `x = x or {}`) read and write the right variable.
local Leaderboard = {}
function Leaderboard.record(kills, deaths)
	local gained = kills * 10
	local lost = deaths * 4
	Leaderboard.last = gained
	Leaderboard.last = lost
	return gained - lost
end
function Leaderboard.snapshot(full)
	if full then
		local rows = { full, full, full }
		Leaderboard.rows = rows
		print(#rows)
	end
	local rows = { "solo" }
	Leaderboard.rows = rows
	return #rows
end
print(Leaderboard.record(3, 2), Leaderboard.last)
print(Leaderboard.snapshot(true), Leaderboard.snapshot(false), #Leaderboard.rows)

local Timer = {}
Timer.__index = Timer
function Timer.new(start)
	return setmetatable({ t = start }, Timer)
end
function Timer:advance(dt, frames)
	for _ = 1, frames do
		self.t += dt
	end
	return self.t
end
local function prefix(tag)
	return function(body)
		return function(suffix)
			return tag .. ":" .. body .. ":" .. suffix
		end
	end
end
local function lattice(n)
	local sum = 0
	for x = 1, n do
		for y = 1, n do
			for z = 1, n do
				sum += x * 100 + y * 10 + z
			end
		end
	end
	return sum
end
print(Timer.new(1):advance(0.5, 4), prefix("a")("b")("c"), lattice(2))

local Emote = {}
function Emote.limiter(window)
	local lastUse = 0
	local count = 0
	return function(cmd)
		if cmd == "/cancel" then
			return "cancelled"
		elseif not (os.clock() - lastUse < window) then
			lastUse = os.clock()
			count += 1
			return "played " .. count
		end
		return "spam"
	end
end
local open = Emote.limiter(-1)
print(open("/wave"), open("/wave"), open("/cancel"))
print(Emote.limiter(1e12)("/wave"))

local trail = {}
local function mark(...)
	trail[#trail + 1] = table.concat({ ... }, "|")
end
local function encode(buffer, cursor, head, body)
	mark("at", cursor)
	cursor += 4
	mark(cursor, head)
	cursor += #body
	mark(cursor, body)
	cursor *= 2
	return cursor, cursor - 1
end
print(encode(nil, 2, "h", "abc"))
local function configure(opts)
	opts = opts or {}
	opts.volume = (opts.volume or 5) + 1
	return opts.volume, opts.muted
end
print(configure(nil), configure({ volume = 9, muted = true }))
local function keepOld(level)
	local raised = level + 3
	mark(level, raised, raised * raised)
	return level, raised
end
print(keepOld(4))
local function observed(level)
	local look = function()
		return level
	end
	level += 1
	mark(level, look())
	level *= 5
	return level, look()
end
print(observed(2))
local function suffixed(name)
	name = name .. "_v2"
	mark(name, #name)
	name = name .. "_final"
	return name, #name
end
print(suffixed("map"))
print(table.concat(trail, " "))
