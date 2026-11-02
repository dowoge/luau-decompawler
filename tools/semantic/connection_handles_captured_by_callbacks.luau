-- A connection handle its own callback disconnects is stored only where the source
-- stores it; a closure used as a table key still writes the local it captures.
local log = {}
local Stepped = { fns = {} }
function Stepped:Connect(fn)
	local slot = #self.fns + 1
	self.fns[slot] = fn
	return {
		Disconnect = function()
			log[#log + 1] = "off" .. slot
			self.fns[slot] = false
		end,
	}
end
function Stepped:Fire(dt)
	for _, fn in ipairs(self.fns) do
		if fn then
			fn(dt)
		end
	end
end

local Tween = {}
function Tween.play(length, opts)
	local link = nil
	local clock = 0
	local hold = opts.hold or 0
	link = Stepped:Connect(function(dt)
		clock += dt
		if clock <= hold then
			return
		end
		log[#log + 1] = "a" .. clock
		if clock >= length then
			link:Disconnect()
		end
	end)
end
function Tween.blink(reset, opts)
	local link
	if reset then
		link = nil
		log[#log + 1] = "reset"
	end
	local hold = opts.hold or 0
	local frames = 0
	local function onStep()
		frames += 1
		if frames > hold and frames >= opts.limit and link then
			link:Disconnect()
			link = nil
		end
	end
	link = Stepped:Connect(onStep)
end
Tween.play(4, { hold = 1 })
Tween.blink(true, { hold = 2, limit = 3 })
for _ = 1, 5 do
	Stepped:Fire(1)
end
print(table.concat(log, ","))

local Clicked = {}
function Clicked:Connect(fn)
	self.fn = fn
	fn()
	return {
		Disconnect = function()
			log[#log + 1] = "unclick"
		end,
	}
end
local function bindOnce()
	local conn
	conn = Clicked:Connect(function()
		if conn then
			conn:Disconnect()
		end
		log[#log + 1] = "click"
	end)
	Clicked.fn()
end
local function bindLoose()
	local conn
	local other = Clicked:Connect(function()
		if conn then
			conn:Disconnect()
		end
		log[#log + 1] = "loose"
	end)
	Clicked.fn()
	conn = nil
	return other ~= nil
end
bindOnce()
print(bindLoose(), table.concat(log, ","))

local Joined = {}
function Joined:Connect(fn)
	self.fn = fn
	return {
		Disconnect = function()
			self.fn = nil
		end,
	}
end
local lobby = {}
local joinConn = Joined:Connect(function(player)
	lobby.Host = player
	print("joined", player.Name, lobby.Host == player)
end)
Joined.fn({ Name = "Builder" })
joinConn:Disconnect()
print("unbound", Joined.fn == nil)

local function register(callbacks)
	local state
	callbacks[function()
		state = "ran"
	end] = true
	for cb in callbacks do
		cb()
	end
	local seen = state
	state = nil
	return seen, state
end
print(register({}))
