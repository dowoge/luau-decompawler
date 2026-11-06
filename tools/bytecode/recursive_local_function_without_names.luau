local function callback(n, n2)
    if n <= 0 then
        return n2
    end
    return callback(n - 1, n2 + n)
end

local s = (tostring(1) .. tostring(2)) .. "-"
local v6 = select("#", 1, 2, nil)
print(s, (function(value, value2, value3)
    return (tostring(value) .. tostring(value2)) .. (value3 or "-"), select("#", value, value2, value3)
end)("x", "y", "z"))
print(callback(3, 4), (function(arg1_1, arg2_1)
    return arg2_1, arg1_1
end)("p", "q"))
local t = { v = 5 }

t.add = function(t2, arg2_1)
    t2.v += arg2_1
    return t2.v
end

print(t:add(2), t.add(t, 3))
