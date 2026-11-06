-- `a, b, c = c, a, b` inside a branch: the values still waiting must be read
-- before any of the three is written, or a later one sees a new value.
local function order(a, b, c)
	if a < b then
		if a < c then
		else
			a, b, c = c, a, b
		end
	elseif b < c then
		a, b, c = b, c, a
	else
		a, b, c = c, a, b
	end
	return a + 2 ^ 16 * b + 2 ^ 32 * c
end
for a = 1, 3 do
	for b = 1, 3 do
		for c = 1, 3 do
			print(a, b, c, order(a, b, c))
		end
	end
end

local function rotateOnce(a, b, c, flip)
	if flip then
		a, b, c = c, a, b
	end
	return a, b, c
end
print(rotateOnce(1, 2, 3, true))
print(rotateOnce(1, 2, 3, false))
