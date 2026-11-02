-- Loop exits and heads that carry effects: early and immediate returns, negated and
-- effectful tests, endless coroutine loops, retries, cleanups and one-shot loops.
local log = {}
local function say(v)
	log[#log + 1] = tostring(v)
	return v
end

local function nextCheckpoint(count, goal)
	for cp = 1, count do
		say("cp" .. cp)
		if cp == goal then
			return cp, "reached"
		end
	end
	return 0, "lost"
end
local function firstLap(count)
	for lap = 1, count do
		return lap, lap * 60
	end
	return "no laps"
end
print(nextCheckpoint(0, 1), nextCheckpoint(4, 2), nextCheckpoint(2, 7))
print(firstLap(0), firstLap(3))

local function charge(limit)
	local pct = 0
	while not (pct >= limit) do
		pct += 25
		say(pct)
		if pct == 50 then
			break
		end
	end
	return pct
end
local function drain(limit)
	local tank = 0
	while say(tank) < limit do
		tank += 1
		if tank % 2 == 1 then
			continue
		end
		say("even" .. tank)
	end
	return tank
end
local function untilMatch(code)
	local tries = 0
	while true do
		tries += 1
		if tries == code then
			break
		end
		say("try" .. tries)
	end
	return tries
end
print(charge(0), charge(25), charge(200), drain(0), drain(3), untilMatch(1), untilMatch(3))
print(table.concat(log, ","))

local ticker = coroutine.create(function()
	while true do
		coroutine.yield("tick")
	end
end)
local banker = coroutine.create(function()
	local bank = 0
	while true do
		coroutine.yield("open", bank)
		for _, coin in { 5, 20 } do
			bank += coin
		end
		coroutine.yield("closed", bank)
	end
end)
local timer = coroutine.create(function()
	while true do
		coroutine.yield("ready")
		for s = 3, 1, -1 do
			coroutine.yield("t-" .. s)
		end
		coroutine.yield("go")
	end
end)
for round = 1, 6 do
	print(round, coroutine.resume(ticker), coroutine.resume(banker))
	print(round, coroutine.resume(timer))
end

local function claimName(wanted)
	local attempts = 0
	while true do
		local handle = ""
		for _, part in { "first", "last" } do
			handle ..= wanted[part] or ""
		end
		attempts += 1
		if handle == "" then
			return attempts
		elseif wanted.last then
			wanted.last = nil
		elseif wanted.first then
			wanted.first = nil
		else
			return -attempts
		end
	end
end
print(claimName({}), claimName({ first = "Ab" }), claimName({ first = "Ab", last = "Cd" }))

local function sweep(fast, hits)
	local ignore = { "player" }
	if fast then
		return hits[1] and hits[1].name or "air", hits[1] ~= nil, #ignore
	end
	local base = #ignore
	local n = 0
	repeat
		n += 1
		local hit = hits[n]
		if hit then
			if hit.blocks then
				for i = #ignore, base + 1, -1 do
					ignore[i] = nil
				end
				return hit.name, true, #ignore
			end
			ignore[#ignore + 1] = hit.name
		end
	until not hit
	for i = #ignore, base + 1, -1 do
		ignore[i] = nil
	end
	return "air", false, #ignore
end
print(sweep(true, {}), sweep(true, { { name = "crate" } }))
print(sweep(false, {}))
print(sweep(false, { { name = "glass" }, { name = "wall", blocks = true } }))
print(sweep(false, { { name = "fog" }, { name = "leaf" } }))

local function afkKick()
	local beat, lastInput = 0, 0
	while true do
		beat += 1
		if beat <= 2 then
			lastInput = beat
		else
			local idle = beat - lastInput
			if idle > 3 then
				return beat, idle
			end
			if idle > 1 then
				say("warn" .. idle)
			end
		end
	end
end
print(afkKick())

log = {}
local function gate(level, need)
	if level < need then
		return "locked"
	else
		say("open" .. level)
	end
	return "entered"
end
for _, level in { 1, 9, 0 / 0 } do
	print("gate", gate(level, 5))
end
local function climb(steps, maxStep)
	local at = 0
	repeat
		at += 1
		if at > #steps or maxStep < steps[at] then
			break
		end
		say("step" .. steps[at])
	until #steps <= at
end
climb({}, 3)
climb({ 1, 7, 2 }, 3)
climb({ 2, 3 }, 3)

local round, polls = 0, 0
local function over()
	polls += 1
	return round >= 6
end
while true do
	round += 1
	if round % 3 ~= 0 then
		continue
	end
	say("r" .. round)
	if over() then
		break
	end
end
print("rounds", round, polls)
local lane = 0
while true do
	lane += 1
	for pin = 1, 3 do
		if pin == 2 then
			continue
		end
		say(lane .. "." .. pin)
	end
	if lane >= 2 then
		break
	end
end
for _, skip in { true, false } do
	while true do
		if skip then
			break
		end
		say("shot")
		break
	end
	say("next " .. tostring(skip))
end
local getters = {}
for slot = 1, 2 do
	while true do
		local kept = slot * 11
		getters[#getters + 1] = function()
			return kept
		end
		break
	end
	while true do
		local tostring = function()
			return "mine" .. slot
		end
		say(tostring())
		break
	end
	say(tostring(slot))
end
for _, get in getters do
	say(get())
end
print(table.concat(log, ","))
