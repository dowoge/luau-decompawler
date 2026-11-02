-- a helper inlined with constant arguments loses the branches they decide;
-- the call comes back with the constants it was given
local log = {}
local function note(...)
	log[#log + 1] = table.concat({ ... }, " ")
end
local function announce(loud)
	note("begin", tostring(loud))
	if loud then
		note("shout")
	end
	note("done")
end
local function greet(polite)
	if polite then
		note("bow")
	end
	note("hello", tostring(polite))
	note("wave")
end
local function quiet(flag)
	if flag then
		note("loud")
	else
		note("quiet")
	end
	note("done")
end
local function pick(kind, value)
	if kind == "double" then
		return value * 2
	elseif kind == nil then
		return -value
	end
	return value
end
local function guard(on, n)
	if not on then
		return 0
	end
	return n + 1
end
local function run(x)
	announce(false)
	announce(true)
	announce(x)
	greet(false)
	greet(true)
	greet(nil)
	greet(1)
	quiet(true)
	quiet(false)
	quiet(x)
	note(pick("double", 4), pick(nil, 3), pick("other", 5), pick(x and "double" or nil, 7))
	note(guard(false, 1), guard(true, 1), guard(x, 10))
end
run(true)
run(false)
print(table.concat(log, "\n"))
