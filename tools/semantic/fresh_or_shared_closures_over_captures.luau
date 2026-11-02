-- Whether a factory's literal is one shared closure or a new one per call depends on what it
-- captures; a folded debug flag must not let a local `_` hide the global `_` the body uses.
_ = "underscore"
local Factory = {}
function Factory.reader(...)
	local TRACE = false
	return function()
		if TRACE then
			print("trace")
		end
		return _
	end
end
function Factory.writer(...)
	local TRACE = false
	return function(v)
		if TRACE then
			print("trace")
		end
		_ = v .. "!"
		return _
	end
end
local r1, r2 = Factory.reader(), Factory.reader()
print(r1 == r2, r1(), r2())
local w1, w2 = Factory.writer(), Factory.writer()
print(w1("a"), w2("b"), w1 == w2, _)

local night = tonumber("2") == 2
local skyboxes = { "stars", "clouds" }
local skybox
if night then
	skybox = skyboxes[1]
else
	skybox = skyboxes[2]
end
function Factory.sky(...)
	return function()
		return skybox
	end
end
local s1, s2 = Factory.sky(), Factory.sky()
print(s1 == s2, s1(), s2())

local title = string.upper("arena")
function Factory.title(...)
	return function()
		return title
	end
end
local t1, t2 = Factory.title(), Factory.title()
print(t1 == t2, t1(), t2())
