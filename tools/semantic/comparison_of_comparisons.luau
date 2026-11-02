-- A comparison used as an operand of another comparison keeps its grouping,
-- whether it stands on the left or on the right.
local Cmp = {}
function Cmp.sameSide(a, b, wantEqual)
	return (a == b) == wantEqual
end
function Cmp.crossed(x1, x2, limit)
	return (x1 < limit) ~= (x2 < limit)
end
function Cmp.matchesFlag(flag, a, b)
	return flag == (a <= b)
end
function Cmp.agree(p, q, r, s)
	return (p > q) == (r > s), (p >= q) ~= (r >= s)
end

print(Cmp.sameSide(1, 1, true), Cmp.sameSide(1, 2, true), Cmp.sameSide("a", "a", false))
print(Cmp.crossed(3, 8, 5), Cmp.crossed(1, 2, 5), Cmp.crossed(9, 7, 5))
print(Cmp.matchesFlag(true, 2, 2), Cmp.matchesFlag(false, 1, 3), Cmp.matchesFlag(true, 5, 4))
print(Cmp.agree(4, 2, 9, 1))
print(Cmp.agree(1, 2, 9, 1))
print(Cmp.agree(3, 3, 2, 2))
