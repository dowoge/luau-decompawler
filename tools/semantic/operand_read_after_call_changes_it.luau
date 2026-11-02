-- Locals read as operands or call receivers after a call that rewrites them, beside
-- helper bodies that read them first: every statement uses the value it reads then.
local mana = 2
local function drain()
	mana = 40
	return 1
end
local function sameBody()
	return mana * drain()
end
local direct = mana * drain()
mana = 2
local opaque = setmetatable({}, { __index = { sameBody = sameBody } })
print(direct, opaque.sameBody(), mana)

local tally = 0
local function setTally(n)
	tally = n // 2
	return tally
end
local function maybeSet(n)
	if n > 0 then
		tally = n * 4
		assert(tally)
	end
	return tally
end
local probe = {}
function probe.touch()
	tally = 77
	return 3
end
function probe.suffix()
	tally = 88
	return "?"
end
print(setTally(9), maybeSet(2), maybeSet(-1))
local seed = tonumber("13")
tally = seed // 2
local a = tally - probe.touch()
print(a, tally)
if seed > 0 then
	tally = seed * 4
	assert(tally)
end
local b = tally - probe.touch()
print(b, tally)
local function viaUpvalue(m)
	tally = m // 2
	return tally .. probe.suffix()
end
print(viaUpvalue(21), tally)

local function speaker(name)
	local s = { name = name }
	function s:say(n)
		print("say", self.name, n)
	end
	return s
end
local host = speaker("first")
local swaps = 0
local function swapHost()
	swaps += 1
	host = speaker("guest" .. swaps)
	return swaps
end
local function announceWith(target)
	target:say(swapHost())
end
for _ = 1, 2 do
	host:say(swapHost())
end
announceWith(host)
announceWith(speaker("visitor"))
print(host.name, swaps)
return sameBody, viaUpvalue
