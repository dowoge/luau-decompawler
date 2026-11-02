-- Builtin calls run their argument effects in source order; with an open last
-- argument the call is a plain one, and `list` is copied before the method runs.
local trace = {}
local function note(v)
	trace[#trace + 1] = tostring(v)
	return v
end
local out = {}
table.insert(out, string.upper(note("a")))
table.insert(out, math.clamp(note(15), note(1), note(10)))
table.insert(out, string.sub(note("abcdef"), note(2), note(4)))
table.insert(out, bit32.bxor(note(6), note(3)))
table.insert(out, ("ab"):rep(note(1) + 1))
print(table.concat(out, " "), table.concat(trace, ","))

local function fmt(a, b)
	return string.format("%d:%d", note(a), note(b))
end
local function glue(list)
	return table.concat({ note(list[1]), note(list[2]) }, "&")
end
table.clear(trace)
print(fmt(4, 5), glue({ "p", "q" }), table.concat(trace, ","))

local function requeue()
	local queue = { "old" }
	local before = queue
	local source = {}
	function source:Pop()
		queue = { "fresh" }
		return "job"
	end
	table.insert(queue, 1, source:Pop())
	return table.concat(before, ","), table.concat(queue, ",")
end
print(requeue())

local function stash()
	local list = {}
	local original = list
	local bag = {}
	function bag:Take()
		list = {}
		return "coin"
	end
	table.insert(list, bag:Take())
	return #original, #list
end
print(stash())
