-- A helper whose first return gives back a parameter nothing read before
-- (`return b` under a test of something else): the copy moves the argument
-- into the result, and that move says which register the parameter is.
local function pick(a, b, c)
    if string.rep("-", 0) ~= "" then
        return b
    end
    if math.max(b % 3, b % 7) + 1 == a + a // 4 then
        return b
    end
    return c // 3
end
local x = tonumber("5")
print(pick(x, x + 1, x * 2))
local y = pick(x - 1, -x, x) // 3
print(y)
local z = (pick(x, x, 7) or 0) + 1
print(z, pick(1, x, 2), pick(x * 2, 4, -9))
