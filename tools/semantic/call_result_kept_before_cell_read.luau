-- `local crit = Dice.roll()` must run before `score` is read: the roll bumps
-- `score` through the inlined `award`, so folding the call into the sum reads a stale value.
local score = 0
local function award(points, kind)
	if kind == "kill" then
		score = score + points
	elseif kind == "assist" then
		score = score + (points and 2 or 0)
	end
	return points
end

local Dice = {}
function Dice.roll()
	award(50, "kill")
	return true
end

local function settle()
	local crit = Dice.roll()
	award(crit, "assist")
	return "settled"
end

print(pcall(settle))
print(score)
