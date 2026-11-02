-- A variable declared before the loop and captured inside it is one cell for
-- all trips, even when each trip overwrites it first: every closure reads the last value.
local function readAll(list)
	local out = {}
	for i, fn in ipairs(list) do
		out[i] = tostring(fn())
	end
	return table.concat(out, ",")
end

local function lapTimes(best, gain)
	local list = {}
	for lap = 1, 3 do
		best = gain * lap
		list[#list + 1] = function()
			return best
		end
	end
	return list
end
print(readAll(lapTimes(0, 5)))

local function teamColors(isRed, players)
	local color
	if isRed then
		color = "red"
	else
		color = "blue"
	end
	local list = {}
	for _, name in players do
		color = name .. "!"
		list[#list + 1] = function()
			return color
		end
	end
	return list, color
end
print(readAll((teamColors(true, { "ann", "bo" }))), (select(2, teamColors(false, {}))))

local function bytes()
	local list = {}
	local hi, lo = string.byte("Zz", 1, 2)
	for k = 1, 2 do
		hi = lo - k
		list[k] = function()
			return hi
		end
	end
	return list
end
print(readAll(bytes()))

local function tiles(rows)
	local list = {}
	for r, width in ipairs(rows) do
		for c = 1, 2 do
			r = width * 10 + c
			list[#list + 1] = function()
				r += 1
				return r
			end
		end
	end
	return list
end
print(readAll(tiles({ 4, 7 })))

local function perTrip(names)
	local list = {}
	for _, name in names do
		local tag = name
		list[#list + 1] = function()
			tag ..= "+"
			return tag
		end
	end
	return list
end
local trips = perTrip({ "p", "q" })
print(trips[1](), trips[1](), trips[2]())
