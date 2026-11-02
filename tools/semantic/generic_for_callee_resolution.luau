-- A local alias of `pairs` drives two later loops, and `for ... in obj` calls
-- __iter only when its loop starts, after every statement before it.
local function census(players, teams)
	local each = pairs
	local count = 0
	for _ in each(players) do
		count += 1
	end
	local names = {}
	for name in each(teams) do
		names[#names + 1] = name
	end
	table.sort(names)
	return count, table.concat(names, "/")
end
print(census({ ann = 1, bo = 2, cy = 3 }, { red = true, blue = true }))

local steps = {}
local level = 0
local bag = setmetatable({}, {
	__iter = function()
		steps[#steps + 1] = "iter@" .. level
		return next, { gem = level }
	end,
})
local function scan()
	steps[#steps + 1] = "start@" .. level
	level = 7
	local total = 0
	for _, v in bag do
		total += v
	end
	level = 9
	steps[#steps + 1] = "end@" .. level
	return total
end
print(scan(), table.concat(steps, ","))
