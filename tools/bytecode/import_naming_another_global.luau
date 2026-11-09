local num = tonumber("-2.7")
local num2 = tonumber("3.2")
print(math.abs(num), math.floor(num2), math.ceil(num))
local str = tostring(5)
print(type(str), str)

local function helper(s, s2)
    local s3 = s .. s2
    local count = #s3
    return s3, count, select("#", s, s2)
end

local n = 10

local function cap(arg1_1)
    n += arg1_1
    return n
end

print(helper("ab", "cd"))
print(cap(1), cap(2))
print(1, 2, pairs ~= nil)
print(2, 4, pairs ~= nil)
local value = string.rep("z", 2)
print(value, math.max(num, num2))
