local function tostring2(value, value2, math)
    local text = tostring(value) .. tostring(value2)
    return text .. (math or "-"), select("#", value, value2, math)
end

local function tostring3(n, n2)
    if n <= 0 then
        return n2
    end
    return tostring3(n - 1, n2 + n)
end

local function tostring4(arg1_1, arg2_1)
    return arg2_1, arg1_1
end

print(tostring2(1, 2, nil), tostring2("x", "y", "z"))
print(tostring3(4, 0), tostring4("p", "q"))
local t = { v = 5 }

t.add = function(t2, arg2_1)
    t2.v += arg2_1
    return t2.v
end

print(t:add(2), t.add(t, 3))
