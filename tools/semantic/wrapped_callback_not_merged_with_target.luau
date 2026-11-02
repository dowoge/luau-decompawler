-- A callback replaced in a branch by a wrapper that captures the original: the wrapper
-- must call the original handler, never itself, and an untouched handler stays as it was.
local function withBonus(bonus, fn)
	return function(x)
		return fn(x) + bonus
	end
end
local function decorate(bonus)
	return function(handler, deliver)
		local final = handler
		if bonus then
			final = withBonus(bonus, handler)
		end
		deliver(final)
	end
end
local function double(x)
	return x * 2
end
local function show(f)
	print(f(10))
end
decorate(3)(double, show)
decorate(nil)(double, show)

local function guarded(tag, work, onOk, onFail)
	return function(...)
		local ok, value = pcall(work, ...)
		if ok then
			onOk(tag, value)
		else
			onFail(tag, value)
		end
	end
end
local function route(tag, okWork, failWork, accept, decline)
	local onSuccess = accept
	if okWork then
		onSuccess = guarded(tag, okWork, accept, decline)
	end
	local onFailure = decline
	if failWork then
		onFailure = guarded(tag, failWork, accept, decline)
	end
	return onSuccess, onFailure
end
local events = {}
local function accept(tag, v)
	events[#events + 1] = "ok " .. tostring(tag) .. " " .. tostring(v)
end
local function decline(tag, v)
	events[#events + 1] = "fail " .. tostring(tag) .. " " .. tostring(v)
end
local s1, f1 = route("Buy", function(x)
	return x + 1
end, function()
	error("refund", 0)
end, accept, decline)
s1(41)
f1(0)
local s2, f2 = route("Sell", nil, nil, accept, decline)
s2("raw")
f2("raw2")
print(table.concat(events, "; "))
