-- loops followed by a jump: the compiler threads their exits through it
local M = {}
function M.forelse(a, b)
    if a then
        for i = 1, 3 do print(i) end
    elseif b then
        print("b")
    end
end
function M.forelse2(a, b)
    if a then
        for i = 1, 3 do print(i) end
    else
        print("else")
    end
    return b
end
function M.forbreak(a, t)
    if a then
        for i = 1, #t do
            if t[i] == 0 then break end
            print(t[i])
        end
    elseif t then
        print("t")
    end
end
function M.fornest(n)
    for i = 1, n do
        if i % 2 == 0 then
            for j = 1, i do print(j) end
        else
            print("odd")
        end
    end
end
function M.forinwhile(n)
    while n > 0 do
        if n > 1 then
            for i = 1, n do print(i) end
        else
            print("one")
        end
        n -= 1
    end
end
function M.forstep(a)
    if a > 0 then
        for i = a, 1, -1 do print(i) end
    elseif a < 0 then
        for i = a, -1 do print(i) end
    else
        print("zero")
    end
end
function M.generic(a, t)
    if a then
        for _, v in ipairs(t) do print(v) end
    elseif t then
        for k in pairs(t) do print(k) end
    end
end
function M.forlast(n)
    for i = 1, n do
        print(i)
    end
end
function M.whileelse(a, n)
    if a then
        while n > 0 do n -= 1 end
    else
        n = 7
    end
    return n
end
function M.repeatelse(a, n)
    if a then
        repeat n -= 1 until n <= 0
    else
        n = 7
    end
    return n
end
function M.tempinregion(a, f)
    local x = a or (if f(1) then 2 else 3)
    return math.abs(x), x
end
function M.tempinregion2(a, f)
    local x = a or (if f(1) > 0 then 5 else 1)
    x = x - 1
    return x
end
M.forelse(true) M.forelse(false, true) M.forelse2(true) M.forelse2(false) M.forbreak(true, {1, 0, 2}) M.forbreak(false, {}) M.fornest(3) M.forinwhile(2)
M.forstep(2) M.forstep(-2) M.forstep(0) M.generic(true, {1}) M.generic(false, {1}) M.forlast(2) print(M.whileelse(true, 2), M.whileelse(false, 2), M.repeatelse(true, 2), M.repeatelse(false, 2))
print(M.tempinregion(nil, function() return true end)) print(M.tempinregion(-5)) print(M.tempinregion2(5), M.tempinregion2(nil, function() return 1 end) == nil)
