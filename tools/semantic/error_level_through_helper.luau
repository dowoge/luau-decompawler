-- Validators raising errors aimed at their caller's caller, inlined into methods: each
-- error still fires with the same text once positions are stripped, good input passes.
local Profile = {}
Profile.__index = Profile

local function inRange(value, low, high, what)
	if type(value) ~= "number" or value < low or value > high then
		error(what .. " out of range", 2)
	end
	return value
end

local function present(t, key)
	if t[key] == nil then
		error("no field " .. key, 1)
	end
end

local function nonEmpty(value)
	if value == "" then
		_G.error("empty value", 2)
	end
end

function Profile.new(name)
	return setmetatable({ name = name, level = 1 }, Profile)
end

function Profile:setLevel(n)
	self.level = inRange(n, 1, 99, "level")
	return self.level
end

function Profile:setVolume(n)
	if type(n) ~= "number" or n < 0 or n > 10 then
		error("volume" .. " out of range", 2)
	end
	self.volume = n
	return n
end

function Profile:rename(name)
	nonEmpty(name)
	self.name = name
	return #name
end

function Profile:title()
	present(self, "name")
	return "[" .. self.name .. "]"
end

function Profile:nick(nick)
	local chosen = nick or _G["error"]("nick required", 2)
	self.nickname = chosen
	return chosen
end

local function badge(value)
	return value or _G["error"]("badge required", 2)
end

local function isRemote(value)
	if not (typeof(value) == "userdata" or (typeof(value) == "table" and typeof(value.FireServer) == "function")) then
		error("expected a remote", 3)
	end
end

local Net = {}
Net.__index = Net
function Net:Send(remote, payload)
	isRemote(remote)
	self.sent = (self.sent or 0) + 1
	return remote:FireServer(payload)
end
local function ask(net, remote, payload)
	isRemote(remote)
	return remote:FireServer("?" .. payload), net.sent
end

local function clean(message)
	return (string.gsub(tostring(message), "^[^:]*:%d+: ", ""))
end

local function try(...)
	local ok, message = pcall(...)
	print(ok, clean(message))
end

local p = Profile.new("ivy")
print(p:setLevel(12), p:setVolume(4), p:rename("ivy2"), p:title(), p:nick("vee"), badge("gold"))
try(p.setLevel, p, 150)
try(p.setVolume, p, -1)
try(p.rename, p, "")
try(p.title, setmetatable({}, Profile))
try(p.nick, p, nil)
try(badge, false)
local remote = {
	FireServer = function(_, payload)
		return "sent " .. payload
	end,
}
local net = setmetatable({}, Net)
print(net:Send(remote, "hi"), ask(net, remote, "who"))
try(function()
	return net:Send("not a remote", 1)
end)
try(ask, net, {}, "x")
