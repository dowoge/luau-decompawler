-- Swapping team slots that closures capture: every read sees the value at its point,
-- and a swap through a temporary must never collapse into two copies of one side.
local function tag(v)
	return "<" .. tostring(v) .. ">"
end

local Match = {}
function Match.rotate(red, blue, rounds)
	local a, b = red, blue
	local getA = function()
		return a
	end
	local log = { tag(a) }
	for _ = 1, rounds do
		a, b = b, a
	end
	log[#log + 1] = tag(getA())
	return table.concat(log, ""), a, b
end
print(Match.rotate("R", "B", 3))
print(Match.rotate("R", "B", 2))

function Match.penalty(score, keep)
	local a = score
	local deduct = function()
		a = a - 5
	end
	local saved
	if keep then
		saved = a
	else
		saved = "none"
	end
	local shown = tag(a)
	deduct()
	return shown, saved, a
end
print(Match.penalty(20, true))
print(Match.penalty(20, false))

function Match.overtime(score, sudden)
	local a = score
	local getA = function()
		return a
	end
	local old = a
	if sudden then
		a = "sudden"
	else
		a = "shootout"
	end
	return tag(old), a, getA()
end
print(Match.overtime(3, false))

function Match.juggle(x, y)
	local a, b, c = x, y, nil
	local readers = {}
	if a then
		readers[1] = function()
			return c
		end
	end
	readers[2] = function()
		return b
	end
	local laps = 0
	repeat
		laps += 1
		b, c = c, b
		for _ = 1, 2 do
			a, b = b, a
		end
	until laps >= 2
	return tostring(a), tostring(b), tostring(c), tostring(readers[1]()), tostring(readers[2]())
end
print(Match.juggle("p", "q"))

function Match.relay(x)
	local a, b, c = x, "bench", x .. "!"
	local order = {}
	for _ = 1, 2 do
		a, c = c, a
		for _ = 1, 3 do
			order[#order + 1] = a
			if #a > 1 then
				break
			end
		end
	end
	local setC
	local n = 0
	while n < 2 do
		n += 1
		setC = function(z)
			c = z
			return c
		end
	end
	return table.concat(order, " "), a, b, c, setC("sub"), c
end
print(Match.relay("k"))

function Match.writerInLoop(x, enabled)
	local a, c = x, nil
	local order = {}
	local writer
	if enabled then
		if a then
			for _ = 1, 2 do
				writer = function(z)
					a = z
					return a
				end
				order[#order + 1] = tostring(c)
				c, a = a, c
			end
		end
	end
	return table.concat(order, " "), tostring(a), tostring(c), writer and writer("w")
end
print(Match.writerInLoop(1, true))
print(Match.writerInLoop(1, false))

function Match.swapCaptured(x, flip)
	local a, b = x, x * 2
	local readB = function()
		return b
	end
	local readA
	if flip then
		local note = tag(a)
		a, b = b, a
		print(note)
	else
		readA = function()
			return a
		end
	end
	return a, b, readB(), readA and readA()
end
print(Match.swapCaptured(5, true))
print(Match.swapCaptured(5, false))
