-- A factory returning two of its own local functions, the second calling the first,
-- inlined where both results are kept: each stays declared, with its own state per call.
local function makeRecorder(sink)
	local count = 0
	local function write(entry)
		sink[#sink + 1] = entry
	end
	local function stamp(tag)
		count += 1
		write(tag .. "#" .. count)
		return count
	end
	return write, stamp
end

local function makeGauge(limit)
	local level = limit // 2
	local function fill(n)
		level = math.min(level + n, limit)
		return level
	end
	return fill, level
end

local function makeWalker(depth)
	local visits = 0
	local function walk(n)
		visits += 1
		if n > 0 then
			return walk(n - 1)
		end
		return visits
	end
	return walk, depth
end

local function session()
	local front, back = {}, {}
	local writeFront, stampFront = makeRecorder(front)
	writeFront("open")
	stampFront("a")
	local writeBack, stampBack = makeRecorder(back)
	writeBack("start")
	stampBack("x")
	stampBack("y")
	stampFront("b")
	print(table.concat(front, " "), table.concat(back, " "))

	local fill, initial = makeGauge(9)
	print(fill(3), fill(3), initial)

	local walk, depth = makeWalker(3)
	print(walk(depth), depth)
	walk = nil
	print(walk == nil)
end

session()
