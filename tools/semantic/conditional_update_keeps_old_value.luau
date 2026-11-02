-- `if a then a = v end` may print as `a = a and v`, but code after the merge that still
-- needs the old value (a swap, a narrowing `math.min`) must read the value it had.
local function order2(x, y)
	return y, x
end
local function settle(x, y, hooked)
	local hooks = {}
	local a, b, c = order2(x, y)
	if a then
		if hooked then
			hooks[1] = function(z)
				c = z
				return c
			end
		else
			a = (tonumber(b) or 0) + 1
		end
	end
	if not hooked then
		c, a = a, c
	end
	return a, b, c
end
print(settle(1, 2, false))
print(settle(nil, 2, false))
print(settle(1, 2, true))
print(settle(1, nil, false))

local function capacity(slotText, bag)
	local cap = nil
	local parsed = slotText and tonumber(slotText)
	if parsed then
		cap = parsed
	end
	local carried
	if type(bag) == "table" then
		carried = tonumber(bag.Amount)
	elseif type(bag) == "number" then
		carried = bag
	end
	if carried then
		if cap then
			cap = math.min(cap, carried)
		else
			cap = carried
		end
	end
	return cap
end
print(capacity("8", 2), capacity("8", { Amount = "20" }), capacity(nil, 6), capacity("3", nil), capacity(nil, nil), capacity("x", 1))
