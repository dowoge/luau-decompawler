-- `b and true or false` inside a condition: the compiler folds the constants,
-- leaving a jump to the body and a dead one after it. `a or b` still reaches
-- the body when b holds.
local function check(low, mark, items)
	local hits = 0
	for _, v in items do
		if v < low or v == mark and true or false then
			hits += 1
		end
		if v ~= low and (v == mark and true or false) then
			hits += 10
		end
	end
	return hits
end

local seen = {}
local probe = {}
function probe.ready(x)
	seen[#seen + 1] = x
	return x > 2
end
local function gate(x, y)
	if probe.ready(x) or probe.ready(y) and true or false then
		return "open"
	end
	return "shut"
end

print(check(3, 5, {1, 5, 7, 3}), check(0, 0, {0, 1}), check(9, 2, {}))
print(gate(1, 4), gate(3, 0), gate(0, 1), table.concat(seen, " "))
