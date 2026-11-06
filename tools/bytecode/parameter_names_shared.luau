local function x(value, value2, value3)
    local text = tostring(value) .. tostring(value2)
    return text .. (value3 or "-"), select("#", value, value2, value3)
end

local function x2(n, n2)
    if n <= 0 then
        return n2
    end
    return x2(n - 1, n2 + n)
end

local function x3(arg1_1, arg2_1)
    return arg2_1, arg1_1
end

print(x(1, 2, nil), x("x", "y", "z"))
print(x2(4, 0), x3("p", "q"))
local t = { v = 5 }

t.add = function(t2, arg2_1)
    t2.v += arg2_1
    return t2.v
end

print(t:add(2), t.add(t, 3))
