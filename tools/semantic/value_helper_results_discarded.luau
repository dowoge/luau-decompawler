-- Value helpers called only for their effects: each copy still runs every effect of the
-- call, an index through a metatable included, and a kept result is still right.
local events = {}
local config = {}

local function retain(x)
	if type(x) == "table" then
		events[#events + 1] = x.label or "table"
	end
	return x
end

local function explain(x)
	if type(x) == "number" then
		events[#events + 1] = "num" .. x
		return x * 10
	end
	return x
end

local function save(key, value)
	config[key] = value
	events[#events + 1] = "save:" .. key
	return VERSION_TAG
end

local lookups = 0
local proxy = setmetatable({}, {
	__index = function(_, key)
		lookups += 1
		return key
	end,
})

local function depthOf(t)
	events[#events + 1] = "probe"
	return t.depth
end

local function job(label)
	return { label = label, at = #events }
end

local function cycle(n)
	retain(job("load"))
	for i = 1, n do
		if i % 2 == 0 then
			retain(job("even" .. i))
		else
			retain(i)
		end
	end
	explain(n)
	explain("static")
	save("speed", n * 2)
	depthOf(proxy)
	depthOf({ depth = n })
	local held = retain(job("final"))
	return held.label, held.at, #events, lookups
end

print(cycle(3))
print(cycle(0))
print(table.concat(events, ","), config.speed)
