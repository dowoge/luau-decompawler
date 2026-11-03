-- A helper whose first test ends in `or true`: the compiler turns its jump
-- to `return b` into a second `return b`. A copy of it has one store of b,
-- and jumps there instead. Rebuilt, every call gives what the copy gave.
local function first(a, b)
    if (b < 5 and (b < 2.25 or not false)) or (math.min(b, b) or 4) <= 3 then
        return b
    end
    if b > math.min(b, b) then
        return b * -1
    end
    a, b = b, a
    print(a, b)
    return b
end
for _, v in {-6, 1, 3, 4, 9} do
    print(first(v + 1, v), first(10, v) + 1)
end
print(first(7, 2.5), first(7, 6))
local list = {}
table.insert(list, first(math.min(5, string.len(tostring(7))) + 3, -6))
table.insert(list, first(1, 1) + 1)
print(#list, list[1], list[2])
