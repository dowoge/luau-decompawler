-- `local f = A.b` reads (and may raise) where it stands; an alias of a script
-- table's field keeps its old function; a replaced library runs __index on fetch.
local ok = pcall(function()
	local make = Factory.create
	print("between")
	return make(1)
end)
print(ok)

local function guarded(enabled)
	local callee = Undefined.run
	if enabled then
		callee()
	end
	return "finished"
end
print((pcall(guarded, false)))

Plugins = {}
local function swapHandlers()
	Plugins.run = function()
		return "first"
	end
	local held = Plugins.run
	Plugins.run = function()
		return "second"
	end
	return held(), Plugins.run()
end
print(swapHandlers())

local function rounded(enabled, x)
	local round = math.round
	if enabled then
		return round(x)
	end
	return x
end
print(rounded(true, 2.6), rounded(false, 2.6))

local logged = {}
local function indexFunction(flag)
	local callee = math.sqrt.field
	if flag then
		callee()
	end
	return "done"
end
local function effectBetween()
	local callee = string.len.extra
	table.insert(logged, "between")
	callee()
end
table.insert(logged, tostring((pcall(indexFunction, false))))
table.insert(logged, tostring((pcall(effectBetween))))
print(table.concat(logged, ","))

local hits = 0
bit32 = setmetatable({}, {
	__index = function(_, key)
		hits += 1
		return key
	end,
})
local function grab()
	local member = bit32.band
	local count = hits
	return member, count, count
end
print(grab())
