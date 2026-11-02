-- `("..."):format(x)` as a statement still converts its argument (running
-- __tostring); it must stay a call, since a backtick string is no statement.
local pings = 0
local Probe = setmetatable({}, {
	__tostring = function()
		pings += 1
		return "probe#" .. pings
	end,
})

local function heartbeat(times)
	for _ = 1, times do
		("tick %*"):format(Probe)
	end
	return pings
end

print(heartbeat(3), tostring(Probe))
