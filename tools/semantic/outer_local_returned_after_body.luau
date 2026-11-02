-- Helpers that end by returning an outer local, copied where the next statement reads
-- it, and one returning a local its own closure reads: every value lines up.
local log = {}
local loader = {
	Load = function(_, name)
		return {
			name = name,
			Start = function(self)
				log[#log + 1] = "start " .. self.name
			end,
		}
	end,
}
local active = nil
local function current(src)
	if src ~= nil then
		active = src:Load("walk")
		assert(active)
		active.Speed = 2
	end
	return active
end

local blocklist = nil
local function blocked()
	if blocklist ~= nil then
		return blocklist
	end
	blocklist = { "spawn" }
	assert(blocklist)
	blocklist[#blocklist + 1] = "vault"
	return blocklist
end

local function freeze()
	if active ~= nil then
		active.Frozen = true
		log[#log + 1] = "freeze"
	end
	return nil
end

local map = {}
function map:route(from, avoid)
	return from .. " avoiding " .. table.concat(avoid, "&")
end

local function survey(src)
	local list = blocked()
	local head = blocked()[1]
	local counted = select("#", blocked())
	if src then
		current(src):Start()
	end
	local speed = active and active.Speed
	return #list, head, counted, speed, map:route("gate", blocked())
end

local function halt()
	freeze()
	return active and active.Frozen
end
print(survey(nil))
print(survey(loader))
print(halt(), table.concat(log, ","))

local stock = {}
local batch = "none"
local function restock(a, b)
	table.clear(stock)
	stock[1], stock[2] = a, b
	batch = "lot" .. a
	return stock
end
local function check(a)
	local s = restock(a, a * 2)
	print(batch, s[1], s[2])
end
check(1)
local function checkBoth()
	check(5)
	check(6)
end
checkBoth()

local Bus = {}
Bus.__index = Bus
function Bus.new()
	return setmetatable({ subs = {} }, Bus)
end
function Bus:Subscribe(fn)
	local sub = { on = true }
	function sub:Cancel()
		self.on = false
	end
	self.subs[#self.subs + 1] = function(...)
		if sub.on then
			fn(...)
		end
	end
	return sub
end
function Bus:Publish(...)
	for _, s in ipairs(self.subs) do
		s(...)
	end
end

local function upTo(bus, limit, fn)
	local sub
	local seen = 0
	sub = bus:Subscribe(function(...)
		seen += 1
		if seen >= limit then
			sub:Cancel()
		end
		fn(...)
	end)
	return sub
end

local got = {}
local bus = Bus.new()
local twice = upTo(bus, 2, function(v)
	got[#got + 1] = "t" .. v
end)
local single = upTo(bus, 1, function(v)
	got[#got + 1] = "o" .. v
end)
single = nil
bus:Publish(1)
bus:Publish(2)
bus:Publish(3)
print(table.concat(got, ","), twice.on, single)
