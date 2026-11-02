-- Locals captured and rewritten inside loops: a swap that stops on a break, and a loop
-- variable rewritten after branchy code, whose closures each keep their own pass.
local function firstThree(list)
	return list[1], list[2], list[3]
end

local function cycle(list, stopAt)
	local lead, middle, last = firstThree(list)
	local resets = {}
	local k = 0
	while k < 2 do
		k += 1
		resets[k] = function(v)
			last = v .. k
			return last
		end
	end
	local turns = 0
	if middle ~= nil then
		while true do
			turns += 1
			last, middle = middle, last
			if turns >= 5 or middle == stopAt then
				break
			end
		end
	end
	local before = middle
	resets[1]("r")
	middle = last
	return lead, before, middle, turns
end
print(cycle({ "a", "b", "c" }, nil))
print(cycle({ "a", "b", "c" }, "c"))
print(cycle({ "a" }, "x"))

local probes = {}
local function walk(a, b, ...)
	local tries = 0
	while true do
		tries += 1
		for _ = 1, 2 do
			if a == b then
				return "met"
			end
			if a > 10 then
				break
			end
			a += 3
		end
		if tries > 2 or (a + b) % 4 == 0 then
			break
		end
		a, b = b, a
	end
	for idx, word in ipairs({ "red", "green", "blue" }) do
		idx = (... or idx) * 100 + tries
		probes[#probes + 1] = function()
			return word .. idx
		end
	end
	return tries
end
print(walk(1, 2), walk(4, 9, 5), walk(2, 2), walk(12, 1))
for i, probe in ipairs(probes) do
	print(i, probe())
end
