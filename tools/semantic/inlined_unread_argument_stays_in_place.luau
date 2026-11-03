-- A helper that never reads a parameter still has its argument evaluated,
-- in order with the others. A pure one stays in the call, in its place; one
-- with effects keeps its place among the other effects.
local log = {}
local function second(a, b, c)
    return b * 2
end
local function noted(x)
    log[#log + 1] = x
    return x
end
local t = {}
local n = tonumber("3")
for i = 1, n do
    t[i] = second(i + 1, i, i * n)
end
print(t[1], t[2], t[3])
print(second(noted(1) + 1, noted(2), -noted(3)), table.concat(log, ","))
for i = 1, 2 do
    t[i] = second(#log + i, i, tostring(noted(i * 10)))
end
print(t[1], t[2], table.concat(log, ","))
