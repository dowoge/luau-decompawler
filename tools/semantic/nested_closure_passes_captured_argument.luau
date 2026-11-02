-- A local read by a closure nested in a closure arrives through the parent's upvalue; written
-- once it is stable for a rebuilt helper call, while one a closure keeps rebinding is not.
local function cooldownLeft(readyAt, now, lag)
	if not readyAt or readyAt <= 0 then
		return 0
	end
	return math.max(0, math.ceil(readyAt - (now + lag)))
end

local Ability = {}
function Ability.schedule(state)
	local lag = state.ping / 100
	local timeline = { cooldownLeft(state.readyAt, state.now, lag) }
	local function onFrame(dt)
		timeline[#timeline + 1] = cooldownLeft(state.readyAt, state.now + dt, lag)
		return function(extra)
			return cooldownLeft(state.readyAt, state.now + dt + extra, lag)
		end
	end
	local preview = onFrame(2)
	local shown = { preview(3), preview(9) }
	return table.concat(timeline, ",") .. "|" .. table.concat(shown, ",")
end
print(Ability.schedule({ readyAt = 30, now = 4, ping = 150 }))

function Ability.jitter(state)
	local lag = 1
	local function spike()
		lag = lag + 6
	end
	local before = cooldownLeft(state.readyAt, state.now, lag)
	spike()
	local after = cooldownLeft(state.readyAt, state.now, lag)
	return before, after, cooldownLeft(nil, 0, lag)
end
print(Ability.jitter({ readyAt = 20, now = 2 }))
