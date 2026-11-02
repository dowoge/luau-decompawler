-- Writes to a local some closure may capture (a swap in a loop, a clamp in an
-- if) must reach that variable: no lost temporary, no fresh select local.
local function rotate(capture, left, right, turns)
	local a, b = left, right
	local peek
	if capture then
		peek = function()
			return b
		end
	end
	for _ = 1, turns do
		a, b = b, a
	end
	return a, b, peek and peek()
end
print(rotate(false, "L", "R", 3))
print(rotate(true, "L", "R", 3))
print(rotate(true, 1, 2, 2))

local reader
local function cap(level, keep)
	if keep then
		reader = function()
			return level
		end
	end
	if level > 10 then
		level = 10
	end
	return level
end
print(cap(25, true), reader())
print(cap(4, true), reader())

local other
local function sign(n, keep)
	if keep then
		other = function()
			return n
		end
	end
	if n >= 0 then
		n = 1
	else
		n = -1
	end
	return n
end
print(sign(9, true), other())
print(sign(-9, true), other())

local watchers = {}
local function climb(steps)
	local height = 0
	for i = 1, steps do
		if i % 2 == 1 then
			watchers[#watchers + 1] = function()
				return height
			end
		end
		if height < i then
			height = i * 3
		end
	end
	return height
end
print(climb(3), watchers[1](), watchers[2]())

local function limit(speed)
	local probe = function()
		return speed
	end
	if speed > 16 then
		speed = 16
	end
	return speed, probe()
end
print(limit(30))
print(limit(8))
