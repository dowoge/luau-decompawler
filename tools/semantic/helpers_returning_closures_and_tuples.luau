-- helpers copied in whole that hand back closures over their own locals, or
-- several values, one of them what they were given
local trace = {}
local function note(...)
	trace[#trace + 1] = table.concat({ ... }, ":")
end

local function meter(start)
	local level = start
	local function raise(by)
		level += by
		return level
	end
	local function read()
		return level
	end
	return raise, read
end

local function tally()
	local seen = 0
	local function hit()
		seen += 1
		return seen
	end
	return hit, seen
end

local function stopwatch()
	local last = 0
	return function(now)
		local gap = now - last
		last = now
		return gap
	end
end

local function labels(list)
	local out = {}
	for i = 1, #list do
		out[i] = list[i] .. "!"
	end
	return out, list
end

local function run(base, words)
	local raise, read = meter(base)
	raise(2)
	note("meter", raise(3), read())
	local other_raise, other_read = meter(base * 10)
	note("other", other_raise(1), other_read(), read())
	local hit, seen = tally()
	hit()
	note("tally", hit(), seen)
	local lap = stopwatch()
	note("lap", lap(5), lap(12), lap(12))
	local named, given = labels(words)
	note("labels", table.concat(named, ","), tostring(given == words))
end

run(1, { "a", "b" })
run(4, { "c" })
print(table.concat(trace, " "))
