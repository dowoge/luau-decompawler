-- conditions with values computed inside them, around closures
local Keys = {PAD = true}
local Ui = {}
function Ui.console() return false end
function Ui.family() return "keyboard" end
local function isPad(k) return k.PAD == true or not Ui.console() end
local M = {}
function M.inlined(a, card)
    if (a and isPad(Keys)) or Keys.PAD == true then
        card.show()
    elseif Ui.family() == "keyboard" then
        print("focus")
    end
    return card
end
function M.inlined2(a, b)
    if a and (b == 1 or not Ui.console()) then
        print("x")
    elseif b then
        print("y")
    end
end
function M.valueinside(a, b, c)
    if (a and (b or c) == 1) or b == 2 then
        print("hit")
    else
        print("miss")
    end
end
function M.withclosures(a, card)
    local count = 0
    local function bump() count += 1 return count end
    local paint
    function card.show()
        if paint then paint() end
    end
    paint = function() print("paint", bump()) end
    if (a and isPad(Keys)) or Keys.PAD == true then
        card.show()
    elseif Ui.family() == "keyboard" then
        print("focus")
    end
    local key = "k" .. tostring(a)
    function card.key() return key, count end
    return card
end
function M.closureafter(a, list)
    local total = 0
    for _, v in list do
        if (a and v > 1) or v == 0 then
            total += v
        elseif v < 0 then
            break
        end
    end
    return function() return total end
end
function M.lateassign(flag)
    local handler
    local run = function(x) return handler and handler(x) end
    if (flag and isPad(Keys)) or flag == 2 then
        handler = function(x) return x * 2 end
    else
        handler = function(x) return x + 1 end
    end
    return run(10)
end
print(M.inlined(true, {show = function() print("shown") end}) ~= nil, M.inlined(false, {show = function() print("shown") end}) ~= nil)
M.inlined2(true, 1) M.inlined2(true, 2) M.inlined2(false, 2) M.inlined2() M.valueinside(1, 1) M.valueinside(1, nil, 2) M.valueinside(nil, 2) M.valueinside()
local c = M.withclosures(true, {}) c.show() print(c.key()) print(M.closureafter(true, {1, 2, 0, -1, 5})(), M.closureafter(false, {2, 0})(), M.lateassign(true), M.lateassign(false), M.lateassign(2))
