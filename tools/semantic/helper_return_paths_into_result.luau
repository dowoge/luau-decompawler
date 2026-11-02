-- Helpers whose returns land in a caller's result: chains of copies, early returns of
-- different locals, boolean paths, and paths that return no value at all.
local function sane(x)
	return x == x and x > -1e9 and x < 1e9
end

local function saneVelocity(v)
	return sane(v[1]) and sane(v[2]) and sane(v[3])
end

local function stepAll(bodies)
	local moved, frozen = 0, 0
	for _, b in ipairs(bodies) do
		if saneVelocity(b.vel) then
			moved += 1
		else
			frozen += 1
		end
	end
	return moved, frozen, saneVelocity(bodies[#bodies].vel)
end
print(stepAll({ { vel = { 1, 2, 3 } }, { vel = { 0 / 0, 0, 0 } }, { vel = { 2e9, 0, 0 } }, { vel = { -4, 5, 6 } } }))

local memo = {}
local function holsterOf(agent)
	if agent ~= nil and agent.Belt ~= nil then
		local belt = agent.Belt
		local hit = memo[agent]
		if hit == nil or hit.On ~= belt then
			local found = belt.Holster
			memo[agent] = found
			return found
		end
		return hit
	end
	return
end

local function weaponName(agent)
	local h = holsterOf(agent)
	return h and h.Weapon
end

local function ammo(agent)
	local h = holsterOf(agent)
	if h == nil then
		return 0
	end
	return h.Rounds
end
local belt = {}
belt.Holster = { On = belt, Weapon = "pistol", Rounds = 6 }
local agent = { Belt = belt }
print(weaponName(agent), ammo(agent), weaponName(agent), ammo({}), weaponName(nil), ammo({ Belt = {} }))

local function invocable(v)
	local kind = type(v)
	if kind == "function" then
		return true
	end
	if kind == "table" then
		local meta = getmetatable(v)
		if meta ~= nil and type(rawget(meta, "__call")) == "function" then
			return true
		end
	end
	return false
end

local function half(n)
	if type(n) == "number" then
		return n / 2
	end
end

local invoker = setmetatable({}, {
	__call = function(_, n)
		return n + 5
	end,
})

local function audit(fn, cfg)
	local results = {}
	if invocable(fn) then
		results[#results + 1] = tostring(fn(10))
	else
		results[#results + 1] = "not run"
	end
	results[#results + 1] = tostring(cfg.spare == nil or invocable(cfg.spare))
	results[#results + 1] = tostring(invocable(cfg.handler))
	local manual
	if cfg.force or type(fn) == "function" then
		manual = true
	elseif type(fn) == "table" then
		local meta = getmetatable(fn)
		manual = meta ~= nil and type(rawget(meta, "__call")) == "function" and true or false
	else
		manual = false
	end
	results[#results + 1] = tostring(manual)
	return table.concat(results, " ")
end

local function mustCall(fn, message)
	assert(invocable(fn), message)
	return "invoker"
end

local function halves(n)
	local r = half(n)
	return select("#", r), r
end

print(audit(function(n)
	return n * n
end, { handler = invoker }))
print(audit(invoker, { spare = 4, force = 1 }))
print(audit(false, { spare = print, handler = 3 }))
print(mustCall(print, "?"), (pcall(mustCall, "text", "no")), halves(9), halves("x"))

local function find(list, key)
	for i = 1, #list do
		if list[i] == key then
			return i
		end
	end
end

local function approve(x)
	if x then
		return "approved"
	end
	print("rejected")
end

local function located(list)
	local at = find(list, "x")
	return at
end

local function locatedLoud(list)
	local at = find(list, "x")
	print("at", at)
	return at
end

local hit = find({ "a", "x" }, "x")
local miss = find({ "a" }, "x")
print(hit, miss, select("#", find({}, "x")))
print(select("#", located({})), select("#", located({ "x" })), select("#", locatedLoud({ "y" })))
if not approve(false) then
	print("so nothing")
end
