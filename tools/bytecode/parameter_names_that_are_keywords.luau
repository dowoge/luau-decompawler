local function callback(value, value2, value3)
    local text = tostring(value) .. tostring(value2)
    return text .. (value3 or "-"), select("#", value, value2, value3)
end

local function callback2(n, n2)
    if n <= 0 then
        return n2
    end
    return callback2(n - 1, n2 + n)
end

local function callback3(arg1_1, arg2_1)
    return arg2_1, arg1_1
end

print(callback(1, 2, nil), callback("x", "y", "z"))
print(callback2(4, 0), callback3("p", "q"))
local t = { v = 5 }

t.add = function(t2, arg2_1)
    t2.v += arg2_1
    return t2.v
end

print(t:add(2), t.add(t, 3))
