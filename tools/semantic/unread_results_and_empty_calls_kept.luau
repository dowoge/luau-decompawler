-- Copied-in calls whose value nothing reads. A call made as a statement
-- compiles a constant, local or global it returns to nothing, so a copy that
-- loads its result was `local x = f()`; one returning something computed may
-- be either, and stays a statement. `(function() end)()` leaves no code at
-- all, and comes back where its line says.

(function() end)()

local unread = (function()
	return ""
end)()

local count = 0
local function bump()
	count += 1
	return count
end

local function work(n)
	local kept = (function()
		return n
	end)()
	local g = (function()
		return print
	end)()
	bump()
	local x = (function()
		return n * 2
	end)()
	return bump() + n
end

print(work(3), work(4), count)
