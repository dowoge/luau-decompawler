-- A recursive local function made in one arm of a branch and taken as that branch's value
-- must still call itself, never a nil placeholder, in either arm and either shape.
local Path = {}
function Path.solver(useBfs, fallback)
	local solve = fallback
	if useBfs then
		local function hops(n)
			if n <= 1 then
				return "arrived"
			end
			return hops(n - 2)
		end
		solve = hops
	end
	return solve
end
print(Path.solver(true, nil)(5), Path.solver(false, function()
	return "teleport"
end)(5))

function Path.walker(flying)
	local move
	if flying then
		local function glide(height)
			if height <= 0 then
				return "landed"
			end
			return glide(height - 3)
		end
		move = glide
	else
		move = function()
			return "walked"
		end
	end
	return move(7)
end
print(Path.walker(true), Path.walker(false))

function Path.unlessBlocked(blocked, detour)
	local route = detour
	if not blocked then
		local function climb(step)
			if step >= 4 then
				return "summit"
			end
			return climb(step + 1)
		end
		route = climb
	end
	return route(0)
end
print(Path.unlessBlocked(false, nil), Path.unlessBlocked(true, function()
	return "around"
end))

function Path.costOr(enabled)
	local cost
	if enabled then
		local function tri(n)
			if n <= 0 then
				return 0
			end
			return n + tri(n - 1)
		end
		cost = tri
	else
		cost = -1
	end
	return cost
end
print(Path.costOr(true)(4), Path.costOr(false))
