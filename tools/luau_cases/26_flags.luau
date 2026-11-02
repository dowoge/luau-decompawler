-- a test that needs statements, jumping into the arms of the next if
local Keys = {PAD = true}
local Ui = {}
function Ui.console() return false end
function Ui.family() return "keyboard" end
local function isPad(k) return k.PAD == true or not Ui.console() end
local M = {}
function M.thenarm(a, b)
    if (a and isPad(Keys)) or b == 1 then
        print("first")
    elseif b == 2 then
        print("second")
    end
end
function M.elsearm(a, b)
    if not (a and isPad(Keys)) and b == 1 then
        print("first")
    else
        print("else")
    end
end
function M.middlearm(a, b)
    if b == 1 then
        print("one")
    elseif (a and isPad(Keys)) or b == 2 then
        print("two")
    elseif b == 3 then
        print("three")
    else
        print("other")
    end
end
function M.withlocals(a, b)
    local x = 1
    if a then
        local y = isPad(Keys) and 2 or 3
        x = x + y
        if y == 2 then
            print("jump", x)
            return x
        end
    end
    if b then
        print("b", x)
    end
    return x
end
M.thenarm(true) M.thenarm(false, 1) M.thenarm(false, 2) M.thenarm() M.elsearm(true, 1) M.elsearm(false, 1) M.elsearm(false, 2) M.middlearm(nil, 1) M.middlearm(true) M.middlearm(nil, 2) M.middlearm(nil, 3) M.middlearm()
print(M.withlocals(true, true), M.withlocals(false, true), M.withlocals())
