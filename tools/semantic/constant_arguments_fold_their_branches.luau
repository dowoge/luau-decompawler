-- a helper inlined with constant arguments loses the branches they decide;
-- the call comes back with the constants it was given
local log = {}
local function note(...)
	log[#log + 1] = table.concat({ ... }, " ")
end
local function helper(flag)
	note("start", tostring(flag))
	if flag then
		note("enabled")
	end
	note("end")
end
local function leading(flag)
	if flag then
		note("first")
	end
	note("then", tostring(flag))
	note("last")
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
	helper(false)
	helper(true)
	helper(x)
	leading(false)
	leading(true)
	leading(nil)
	leading(1)
	quiet(true)
	quiet(false)
	quiet(x)
	note(pick("double", 4), pick(nil, 3), pick("other", 5), pick(x and "double" or nil, 7))
	note(guard(false, 1), guard(true, 1), guard(x, 10))
end
run(true)
run(false)
print(table.concat(log, "\n"))
