local num = tonumber("-2.7")
local value = tonumber("3.2")
print(math.abs(num), math.floor(value), math.ceil(num))
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
local value2 = string.rep("z", 2)
print(value2, math.max(num, value))
