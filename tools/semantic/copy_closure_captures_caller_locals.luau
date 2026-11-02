-- Inlined helpers whose function literals capture the caller's locals: each literal sees
-- the real argument, and writes made through a captured local reach the caller.
local unlocked = {}
local function withPlayer(player, fn)
	if not player then
		return false
	end
	local ok = pcall(function()
		fn(player, #unlocked)
	end)
	return ok
end
local function grant(name, count)
	unlocked[#unlocked + 1] = name .. "@" .. count
end

local report = {}
for _, p in ipairs({ "ann", false, "cy" }) do
	report[#report + 1] = tostring(withPlayer(p, grant))
	if p then
		withPlayer(p, grant)
	end
end
report[#report + 1] = tostring(withPlayer(nil, grant))
print(table.concat(report, ","), table.concat(unlocked, ";"))

local pending = {}
local function defer(task)
	pending[#pending + 1] = task
	return task
end

local function session(name)
	local closes = 0
	defer({
		run = function()
			closes += 1
		end,
	})
	pending[#pending].run()
	pending[#pending].run()
	return name .. ":" .. closes
end
print(session("one"), session("two"), #pending)

local function makeDoor()
	local latch = nil
	local swings = 0
	local function setOpen(open)
		if not open then
			if latch then
				latch.open = false
				latch = nil
			end
			return
		end
		swings += 1
		latch = latch or { open = true }
	end
	return function(want, jammed)
		if jammed then
			setOpen(false)
		else
			setOpen(want)
		end
		return tostring(latch ~= nil) .. swings
	end
end
local door = makeDoor()
local states = {}
for _, step in ipairs({ { true, false }, { true, true }, { false, false }, { true, false }, { true, false }, { false, true } }) do
	states[#states + 1] = door(step[1], step[2])
end
print(table.concat(states, " "))

local function score(n, bonus)
	local raw = n * bonus
	if raw < 0 then
		raw = 0
	end
	local total = raw + 10
	return total, total > 20 and "high" or "low"
end
local best, why = -1, "unset"
local rows = {}
repeat
	best = score(#rows + 1, 8)
	rows[#rows + 1] = best .. why
	best, why = score(-#rows, #rows)
	rows[#rows + 1] = best .. why
until #rows >= 4
local single = score(3, 3)
print(table.concat(rows, " "), best, why, single)
