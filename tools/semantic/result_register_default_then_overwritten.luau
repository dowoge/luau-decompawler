-- A helper whose result takes a default first and is overwritten only when a check
-- passes, with a failure injected at each effect: order and results must match.
local trace, effectNo, failAt = {}, 0, 0
local function effect(label)
	effectNo += 1
	trace[#trace + 1] = label
	if effectNo == failAt then
		error("fail@" .. label, 0)
	end
end

local now, skew, allow = 0, 0, false
local Clock = {
	read = function()
		effect("read")
		now += 5
		return now, "late"
	end,
}
local profileMeta = {
	__index = function(_, k)
		effect("field:" .. k)
		return skew
	end,
}
local Accounts = {
	open = function(_, who)
		effect("open")
		trace[#trace + 1] = tostring(who)
		return setmetatable({}, profileMeta)
	end,
}
local Rules = {}
function Rules.admit(_, v)
	effect("admit")
	trace[#trace + 1] = tostring(v)
	if Rules.peek then
		trace[#trace + 1] = "saw:" .. tostring(Rules.peek())
	end
	return allow
end

local function serverTime(who)
	if not who then
		return Clock.read()
	end
	local profile = Accounts:open(who)
	local base = Clock.read()
	local shifted = base + profile.Skew
	if Rules:admit(shifted) then
		return shifted
	end
	return base
end

local function stampFor(who)
	local t = serverTime(who)
	return { value = t, n = select("#", t) }
end

local function stampWatched(who)
	local t
	Rules.peek = function()
		return t
	end
	if not who then
		t = Clock.read()
	else
		local profile = Accounts:open(who)
		t = Clock.read()
		local shifted = t + profile.Skew
		if Rules:admit(shifted) then
			t = shifted
		end
	end
	Rules.peek = nil
	return { value = t, n = select("#", t) }
end

local cases = {}
for _, who in ipairs({ false, "ana", 3 }) do
	for _, s in ipairs({ 0, 15, -math.huge }) do
		for _, ok in ipairs({ true, false }) do
			cases[#cases + 1] = { who = who, skew = s, ok = ok }
		end
	end
end
for _, c in ipairs(cases) do
	for fail = 0, 4 do
		for _, fn in ipairs({ stampFor, stampWatched }) do
			trace, effectNo, failAt = {}, 0, fail
			now, skew, allow = 100, c.skew, c.ok
			Rules.peek = nil
			local good, res = pcall(fn, c.who)
			local tail = good and (tostring(res.value) .. " " .. res.n) or ("error " .. res)
			print(tostring(c.who), c.skew, c.ok, fail, tail, table.concat(trace, ","))
		end
	end
end
failAt = 0
print("counts", select("#", serverTime(nil)), select("#", serverTime("x")))
