-- `local ok = s.on and f(a) and g(b)` after values made by calls: the and
-- chain's first read was written out before those calls (`local ok =
-- s.on` above them), so it read state they had not set yet. Values with
-- effects made before the chain go out first.

local state = { on = false, n = 0 }
local log = {}
local function turn(v)
	state.on = v
	state.n += 1
	log[#log + 1] = "turn" .. tostring(v)
	return state.n
end
local function near(x, y)
	log[#log + 1] = "near"
	return math.abs(x - y) < 1
end

local function check(v)
	local a = turn(v) * 2
	local b = turn(not v) + a
	local ok = state.on and near(a, 2) and near(b, 3)
	if ok then
		ok = near(b, b)
	end
	return ok, a, b
end

print(check(true))
print(check(false))
print(table.concat(log, ","))

local cache = {}
local function key(k, v)
	cache[k] = v
	cache.last = k
	return v
end
local function lookup(t)
	local first = cache.a or key("a", 1)
	local second = t[1]
	local third = cache.last
	if not third then
		third = key("c", 3)
	end
	return first, second, third
end
print(lookup({ 9 }))
print(lookup({ 8 }))
