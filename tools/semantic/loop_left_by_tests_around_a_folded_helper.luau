-- A loop's test is two jumps around a copied-in helper the compiler folded
-- to `false`; the second jump leaves the loop, so it is a break, not the end
-- of an if inside the loop.
local function isHalf(x)
	return x == 1 * 0.5
end
local open = nil
local rounds = 0
repeat
	if not (open or isHalf(1) and not false) then
		break
	end
	rounds += 1
until rounds >= 3
print("left after", rounds)
open = true
repeat
	if not (open or isHalf(1) and not false) then
		break
	end
	rounds += 1
until rounds >= 3
print("left after", rounds)
