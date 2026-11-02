-- An empty helper the compiler copies in takes an `if` expression with calls
-- in it, inside one arm of `x and y or z`: computed and never read, the value
-- still runs only when that arm does, and where it does.
local seen = {}
function weigh(a, b, c)
	seen[#seen + 1] = "weigh"
	return (if a then 1 else 0) + b + c
end
function probe(x)
	seen[#seen + 1] = "probe"
	return x > 1
end
function tag(s)
	seen[#seen + 1] = s
	return s
end
level, base = 2, 3
local function ignore(_, _, _) end

local result = if 1 * -1 ~= weigh((probe(1) and 1 * 3 or ignore(not 1, 100, if tag("x") == `<{level}:{"a"}>` .. (if false then "q" else "a") then tag("then") else tag("else"))) == level, (if 1 then base else 2.25) - base, base) then 1 % 5 else nil
print(result, table.concat(seen, " "))
