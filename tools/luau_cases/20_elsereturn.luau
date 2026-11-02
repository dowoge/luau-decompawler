-- an `else` before the function's end: the jump over it becomes a return
local M = {}
function M.basic(a)
    if a then
        print("then")
    else
        print("else")
    end
end
function M.innerif(a, b)
    if a then
        print("a")
        if b ~= "" then
            print(b)
        end
    else
        print("not a")
    end
end
function M.deep(a, b, c)
    if a then
        if b then
            print("b")
            if c then
                print("c")
            end
        else
            print("not b")
        end
    else
        print("not a")
    end
end
function M.guardinside(a, b)
    if a then
        if b then
            print("early")
            return
        end
        local q = b or "x"
        if q ~= "" then
            print(q)
        end
    else
        print("no")
    end
end
function M.animate(state, t)
    if state.on then
        if state.time > 0 then
            state.time -= t
            return
        end
        local queued = state.queued
        if queued ~= "" then
            print("next", queued)
        end
    else
        print("stop")
        state.time = 0
    end
end
function M.elseifs(a)
    if a == 1 then
        print(1)
    elseif a == 2 then
        if a then print("two") end
    elseif a == 3 then
        print(3)
    end
end
function M.loopthen(a, t)
    if a then
        for _, v in t do
            if v then print(v) end
        end
    else
        print("none")
    end
end
function M.closurethen(a)
    local f
    if a then
        f = function() return a end
        if f() then print("called") end
    else
        print("no closure")
    end
end
function M.compound(a, b, c)
    if a and (b or c) then
        print("yes")
    elseif c then
        print("c")
    end
end
function M.padkeys(s, pad, console, keyboard)
    if s.inner then
        if pad == true or not console then
            s.show()
            return
        end
    end
    if pad == true then
        s.show()
    elseif keyboard then
        print("focus")
    end
    return s
end
M.basic(true) M.basic(false) M.innerif(true, "x") M.innerif(true, "") M.innerif(false) M.deep(1, 1, 1) M.deep(1, 1) M.deep(1) M.deep() M.guardinside(true, true) M.guardinside(true, false) M.guardinside(false)
M.animate({on = true, time = 1}, 1) M.animate({on = true, time = 0, queued = "q"}, 1) M.animate({on = true, time = 0, queued = ""}, 1) M.animate({on = false, time = 1}, 1)
M.elseifs(1) M.elseifs(2) M.elseifs(3) M.elseifs(4) M.loopthen(true, {1, false}) M.loopthen(false) M.closurethen(true) M.closurethen(false) M.compound(1, 1) M.compound(1, nil, 1) M.compound(nil, nil, 1) M.compound()
local s = {inner = true, show = function() print("show") end} print(M.padkeys(s, true) == nil, M.padkeys({show = s.show}, true) ~= nil, M.padkeys({}, false, true, true) ~= nil)
