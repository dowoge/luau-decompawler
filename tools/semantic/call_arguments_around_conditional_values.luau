-- Arguments with effects before a conditional value in the same call, and
-- items before one in a constructor, stay in the call (or the table), and
-- run in their order. `a or b` and `a and b` of plain values call nothing.
local log = {}
local function note(x)
    log[#log + 1] = x
    return x
end
local v = tonumber("3")
local w = tonumber("4")
print(v or w, v and string.len(tostring(v)))
print(tostring(note(1)), math.max(#log, 1), if #log > 0 then "x" else "y")
local t = { tostring(note(2)), note(3) + 1, if #log > 2 then "big" else "small" }
print(t[1], t[2], t[3], table.concat(log, ","))
local function line(a, b, c, d)
    return a .. b .. c .. tostring(d)
end
for i = 1, tonumber("2") do
    print(line("<", tostring(note(i * 10)), tostring(note(i)), if i > 1 then "two" else "one"))
end
print(table.concat(log, ","))
