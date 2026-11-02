-- A copy of a variable a closure changes, taken before a table is built, is
-- the key it stores under: Luau reads a local key only when it stores it.
local function build(key, after, invoke)
	local function change()
		key = after
		return 9
	end
	local function inner()
		local saved = key
		return { [saved] = invoke(change) }
	end
	local t = inner()
	return t.before, t.after
end
print(pcall(build, "before", "after", function(f)
	return f()
end))

local function tally(label, step)
	local count = 0
	local function bump()
		count += step
		return count
	end
	local function snapshot()
		local was = count
		return { seen = was, now = bump(), [label] = was }
	end
	local s = snapshot()
	return s.seen, s.now, s[label]
end
print(tally("x", 2))
