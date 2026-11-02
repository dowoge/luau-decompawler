-- Calls whose last argument can give several values, next to locals holding one: a
-- trailing nil after such a call stays, and a one-value local never becomes the call.
local lines = {}
local function log(...)
	local n = select("#", ...)
	local parts = {}
	for i = 1, n do
		parts[i] = tostring((select(i, ...)))
	end
	lines[#lines + 1] = n .. ":" .. table.concat(parts, ",")
end

local function both()
	return "L", "R"
end

local function place(item, flag)
	log("place", item)
	if flag then
		log("marker", flag)
	end
	log("done", item)
end

local function count2(item, spare)
	log("tally", item)
	log("seen", item)
end

local function stage(...)
	count2(both())
	place(both(), 0)
	local extra = select("#", ...)
	place(..., nil)
	place(both())
	place(both(), false)
	if extra > 0 then
		place(both(), nil)
	end
end
stage("v1", "v2")
print(table.concat(lines, " "))

local source = {}
function source.duo()
	return 8, 9
end
local function head(a)
	return if a ~= nil then a else "none"
end
local function size(...)
	return table.pack(...).n
end
local function join(a, b)
	return string.format("%s+%s", tostring(a), tostring(b))
end

local function checks()
	local results = {}
	local bag = {}
	local four = source.duo()
	table.insert(bag, four)
	local second = source.duo()
	results[#results + 1] = math.max(1, second)
	results[#results + 1] = string.rep("z", second, "")
	local one = source.duo()
	results[#results + 1] = join(one)
	results[#results + 1] = size(one)
	results[#results + 1] = head(one)
	local three = source.duo()
	results[#results + 1] = join(three, three)
	results[#results + 1] = tostring(second)
	results[#results + 1] = #bag
	results[#results + 1] = size(source.duo())
	results[#results + 1] = join(source.duo())
	print(table.concat(results, " "))
end
checks()
