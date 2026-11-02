-- tests with statements in them, whose failure lands in a later arm
local Keys = {PAD = true}
local function usable(k, last)
    if string.find(last, "^Pad") ~= nil then
        return k.PAD
    end
    return false
end
local M = {}
function M.tomake(a, last)
    if a and not usable(Keys, last) then
        print("make")
    else
        print("else")
        print(a)
    end
    print("end")
end
function M.elsewithlocals(a, last)
    if a and not usable(Keys, last) then
        local x = "made"
        print(x)
    else
        local y = tostring(a)
        print(y .. "!")
    end
end
function M.laterarm(a, b, last)
    if a and not usable(Keys, last) then
        print("one")
    elseif b and not usable(Keys, last) then
        print("two")
    else
        print("three")
    end
end
function M.valuearm(a, last)
    local r
    if a and not usable(Keys, last) then
        r = 1
    else
        r = 2
    end
    return r
end
function M.inloop(list, last)
    local n = 0
    for _, v in list do
        if v > 0 and not usable(Keys, last) then
            n += v
        else
            n -= 1
            if n < -2 then break end
        end
    end
    return n
end
function M.returnsarm(a, last)
    if a and not usable(Keys, last) then
        return "early"
    else
        print("else arm")
    end
    return "late"
end
function M.nestedarm(a, b, last)
    if a then
        if b and not usable(Keys, last) then
            print("inner")
        else
            print("inner else")
        end
        print("after inner")
    else
        print("outer else")
    end
end
function M.closurearm(a, last)
    local fs = {}
    if a and not usable(Keys, last) then
        fs[1] = function() return a end
    else
        local z = last
        fs[1] = function() return z end
    end
    return fs[1]()
end
M.tomake(true, "Pad") M.tomake(true, "x") M.tomake(false, "x") M.elsewithlocals(true, "x") M.elsewithlocals(true, "Pad") M.elsewithlocals(false, "x")
M.laterarm(true, false, "x") M.laterarm(false, true, "x") M.laterarm(true, true, "Pad") M.laterarm(false, false, "x")
print(M.valuearm(true, "x"), M.valuearm(true, "Pad"), M.valuearm(false, "x"), M.inloop({1, -1, 2}, "x"), M.inloop({1, 2, 3, 4}, "Pad"), M.returnsarm(true, "x"), M.returnsarm(true, "Pad"), M.returnsarm(false, "x"))
M.nestedarm(true, true, "x") M.nestedarm(true, true, "Pad") M.nestedarm(true, false, "x") M.nestedarm(false) print(M.closurearm(true, "x"), M.closurearm(true, "Pad"), M.closurearm(false, "y"))
