-- The compiler copies a small function's body into another that calls it, so
-- no upvalue links the two; put back, the call names it, and it has to be
-- declared first. Here both come from a function copied into its caller.
local function makePair(start)
	local total = start
	local function count(items)
		local n = 0
		for _ in items do
			n += 1
		end
		return n
	end
	local function addAll(a, b)
		total += count(a) + count(b)
		return total
	end
	return count, addAll
end

local function run(seed)
	local count, addAll = makePair(seed)
	print(count({ 1, 2 }))
	print(addAll({ 3 }, { 4, 5 }))
end
run(10)
