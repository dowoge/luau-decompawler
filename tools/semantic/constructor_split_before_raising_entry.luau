-- An entry that may raise (`//`) before a branch value splits the constructor;
-- its list items, nil ones too, keep their slots, and `[0]`/`[-0]` stay one key.
local function slots(step, base, alt)
	local t = { [0] = base // step, [1] = (if step > 100 then alt else step), nil, "tail" }
	local count = 0
	for _ in next, t do
		count += 1
	end
	return #t, t[0], t[1], t[2], t[3], count
end
print(slots(4, 10, "alt"))
print(slots(400, 10, "alt"))
print((pcall(slots, "x", 10, "alt")))

local function zeroSpellings(flag, a, n)
	local t = { [0] = nil, mode = "m", [-0] = ("%*/%*"):format(a, n), big = n > 99, tostring(flag), nil, math.ceil(n) }
	t[0] = "zero"
	t[-0] = math.ceil(a)
	for key, value in pairs(t) do
		if type(key) == "number" and key == 0 then
			print(1 / key, value)
		end
	end
	return t[1], t[3], t.big, t.mode
end
print(zeroSpellings(false, -2.5, 3.2))
