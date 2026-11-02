-- A local captured by a closure in one branch feeds numeric for loops in the
-- other: copies of it on the way into a loop must happen before the loop.
local saved
local trace = {}

local function stage(level, keep)
	local depth = level
	if level > 2 then
		depth, level = level - 1, depth
	end
	if keep then
		saved = function(v)
			depth = v
			return depth
		end
	else
		local start = depth
		for i = start, start + 1 do
			for _ = 1, 2 do
				if saved then
					trace[#trace + 1] = saved(depth + i)
				end
			end
		end
		for _ = depth, 0 do
		end
	end
	return depth, level
end

print(stage(4, true))
print(stage(5, false))
print(saved(0), table.concat(trace, ","))
