-- A helper returning a function over its argument, inlined with constants and with
-- top-level locals: each run of a copy makes a new function; shared literals stay one.
local function always(v)
	return function()
		return v
	end
end

local function silent()
	return function()
		return nil
	end
end

local report = {}
local function note(what, value)
	report[#report + 1] = what .. "=" .. tostring(value)
end

local handlers = {}
for i = 1, 2 do
	handlers[i] = { on = always("ping"), off = silent() }
end
note("fresh", handlers[1].on == handlers[2].on)
note("shared", handlers[1].off == handlers[2].off)
note("value", handlers[2].on())

local seen, distinct = {}, 0
local k = 0
while k < 3 do
	k += 1
	local fn = always(0)
	if not seen[fn] then
		seen[fn] = true
		distinct += 1
	end
end
note("distinct", distinct)

local function doubler(n)
	local function twice()
		return n + n
	end
	return twice
end
local d1, d2 = doubler(4), doubler(4)
note("named", d1 == d2)
note("named value", d1() + d2())

local region = string.rep("ab", #handlers)
local copied, literal = {}, {}
for i = 1, 3 do
	copied[i] = always(region)
	literal[i] = function()
		return region
	end
end
note("copy loop", copied[1] == copied[3])
note("literal loop", literal[1] == literal[2])
note("copy value", copied[2]())

local function twoDeep(v)
	return function()
		return function()
			return v .. "!"
		end
	end
end
local deep, flat = {}, {}
for i = 1, 2 do
	deep[i] = twoDeep(region)
	flat[i] = function()
		return region .. "!"
	end
end
note("deep", deep[1] == deep[2])
note("flat", flat[1] == flat[2])
note("deep inner", deep[1]() == deep[1]())
note("deep value", deep[2]()())

local motto = tostring(#copied * 7)
local factory = {}
local built = 0
function factory.spawn()
	built += 1
	return function()
		return motto
	end
end
local mottos = {}
for i = 1, 2 do
	mottos[i] = always(motto)
end
note("motto copies", mottos[1] == mottos[2])
note("factory", factory.spawn() == factory.spawn())
note("motto", mottos[1]() .. factory.spawn()() .. built)
print(table.concat(report, "\n"))
