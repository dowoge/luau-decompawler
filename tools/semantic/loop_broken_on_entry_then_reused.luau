-- A numeric loop that always breaks on its first pass never jumps back, and
-- the numeric loop after it reuses its registers.
local function firstAndSum(values)
	local first
	for i = 1, #values do
		first = values[i]
		break
	end
	local sum = 0
	for i = 1, #values do
		sum += values[i]
	end
	return first, sum
end

local function probeThenFill(n, out)
	for _ = 1, n do
		break
	end
	for i = 1, n do
		out[i] = i * i
	end
	return #out
end

print(firstAndSum({ 4, 5, 6 }))
print(firstAndSum({}))
print(probeThenFill(4, {}), probeThenFill(0, {}))
