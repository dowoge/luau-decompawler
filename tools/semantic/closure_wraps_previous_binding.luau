-- A closure built around a variable's previous value and then stored into that
-- same variable keeps calling the previous value, never itself.
local function decorate(prefix, inner)
	return function(text)
		return prefix .. inner(text)
	end
end

local function choose(tag)
	return function(base, sink)
		local handler = base
		if tag then
			handler = decorate(tag, base)
		end
		sink(handler)
	end
end
choose("<")(string.upper, function(h) print("A", h("abc")) end)
choose(false)(string.upper, function(h) print("A", h("abc")) end)

local registry = {}
local function remember(f)
	registry[#registry + 1] = f
	return f
end
local function layered(tag)
	return function(base, sink)
		local handler = base
		if tag then
			handler = remember(function(text)
				return tag .. base(text)
			end)
		end
		sink(handler)
	end
end
layered("#")(string.lower, function(h) print("B", h("XyZ")) end)
layered(nil)(string.lower, function(h) print("B", h("XyZ")) end)

local function stack(n)
	local fn = function(x)
		return x
	end
	for i = 1, n do
		fn = decorate(tostring(i), fn)
	end
	return fn
end
local function stackRepeat(n)
	local fn = function(x)
		return x
	end
	local i = 0
	repeat
		i += 1
		fn = decorate(i .. ":", fn)
	until i >= n
	return fn
end
print("C", stack(3)("z"), stackRepeat(2)("y"))

local function chain(n)
	local fn = function(x)
		return x
	end
	for i = 1, n do
		local prev = fn
		fn = remember(function(x)
			return prev(x) * 2 + i
		end)
	end
	return fn
end
print("D", chain(3)(1), #registry)

local function constant(v)
	return function()
		return v
	end
end
local function trade(a, b, flag)
	local x, y = a, b
	if flag then
		x = constant(b)
		y = a
	end
	return x, y
end
local fx, fy = trade("left", "right", true)
print("E", fx(), fy, trade("left", "right", false))

local function guard(label, fn, onOk, onFail)
	return function(...)
		local ok, res = pcall(fn, ...)
		if ok then
			onOk(res)
		else
			onFail(label, res)
		end
	end
end
local function wire(okHandler, failHandler)
	return function(onOk, onFail)
		local success = onOk
		if okHandler then
			success = guard("ok", okHandler, onOk, onFail)
		end
		local failure = onFail
		if failHandler then
			failure = guard("fail", failHandler, onOk, onFail)
		end
		return success, failure
	end
end
local attempts = 0
local s, f = wire(function(v)
	error("broken " .. v, 0)
end, function(label, e)
	attempts += 1
	if attempts > 5 then
		return "stop"
	end
	error("again", 0)
end)(function(v) print("F ok", v) end, function(label, e) print("F fail", label, e) end)
s("x")
f("y")
print("F attempts", attempts)
