-- A reader made before the loop sees the cell each trip writes; every read is logged into
-- the history list, which must stay that list and not turn into another read.
local function replayInputs(inputs)
	local lastKey = "idle"
	local function sample()
		return lastKey
	end
	local history = {}
	for i = 1, #inputs do
		lastKey = inputs[i]
		table.insert(history, sample())
	end
	return table.concat(history, ">"), sample()
end
print(replayInputs({ "W", "A", "Space" }))
print(replayInputs({}))
