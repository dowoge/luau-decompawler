-- Handlers that forward their varargs into an inlined helper whose first argument is a
-- constant: the spread values reach the right parameters and the extras are dropped.
local board = { Red = {}, Blue = {} }
local history = {}

local function record(team, lane, slot, value)
	if team == "Red" then
		board.Red[lane] = (board.Red[lane] or 0) + (value or 1)
	elseif team == "Blue" then
		local row = board.Blue[lane] or {}
		board.Blue[lane] = row
		row[slot] = value
	end
	history[#history + 1] = team:sub(1, 1) .. lane
end

local function makeLane(lane)
	return {
		red = function(...)
			record("Red", lane, ...)
		end,
		blue = function(...)
			record("Blue", lane, ...)
		end,
	}
end

local lanes = { makeLane(1), makeLane(2) }
lanes[1].red(nil, 4)
lanes[1].red()
lanes[2].blue("left", "flag")
lanes[2].blue("mid", "orb", "surplus")
lanes[1].blue(1)
print(board.Red[1], board.Blue[2].left, board.Blue[2].mid, board.Blue[1][1], table.concat(history, " "))
