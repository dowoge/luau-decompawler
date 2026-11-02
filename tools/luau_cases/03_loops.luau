local M = {}
function M.num(n) local s = 0 for i = 1, n do s += i end return s end
function M.numstep(n) local s = 0 for i = n, 1, -1 do s += i end return s end
function M.numstep2(n) local s = 0 for i = 0, n, 2 do s += i end return s end
function M.numvar(a, b, c) local s = 0 for i = a, b, c do s += i end return s end
function M.pairsloop(t) local n = 0 for k, v in pairs(t) do n += v end return n end
function M.ipairsloop(t) local n = 0 for i, v in ipairs(t) do n += i * v end return n end
function M.iter(t) local n = 0 for k, v in t do n += v end return n end
function M.nextloop(t) local n = 0 for k, v in next, t do n += v end return n end
function M.keyonly(t) local n = 0 for k in pairs(t) do n += 1 end return n end
function M.custom(f) local n = 0 for a, b, c in f() do n += a end return n end
function M.whileloop(n) local i = 0 while i < n do i += 1 end return i end
function M.whiletrue(n) local i = 0 while true do i += 1 if i >= n then break end end return i end
function M.repeatloop(n) local i = 0 repeat i += 1 until i >= n return i end
function M.repeatlocal(n) local i = 0 repeat local d = i * 2 i += 1 until d >= n return i end
function M.repeatand(n, m) local i = 0 repeat i += 1 until i >= n and i >= m return i end
function M.repeator(n, m) local i = 0 repeat i += 1 until i >= n or i >= m return i end
function M.brk(t) for i, v in ipairs(t) do if v == 3 then return i end end return nil end
function M.brk2(t) local r for i, v in ipairs(t) do if v == 3 then r = i break end end return r end
function M.cont(t) local n = 0 for _, v in ipairs(t) do if v == 2 then continue end n += v end return n end
function M.contelse(t)
    local n = 0
    for _, v in ipairs(t) do
        if v == 2 then
            n += 10
            continue
        elseif v == 3 then
            n += 20
        else
            n += 1
        end
        n += 100
    end
    return n
end
function M.contwhile(n)
    local i, s = 0, 0
    while i < n do
        i += 1
        if i % 2 == 0 then continue end
        s += i
    end
    return s
end
function M.contrepeat(n)
    local i = 0
    repeat
        i += 1
        if i == 2 then continue end
        print(i)
    until i >= n
    return i
end
function M.nest(n)
    local s = 0
    for i = 1, n do
        for j = 1, i do
            if j == 3 then break end
            s += j
        end
    end
    return s
end
function M.nestwhile(n)
    local i = 0
    while i < n do
        local j = 0
        while j < i do
            j += 1
            if j > 2 then break end
        end
        i += 1
    end
    return i
end
function M.forwhile(t)
    local out = {}
    for _, v in ipairs(t) do
        local n = v
        while n > 0 do
            n -= 1
            table.insert(out, n)
        end
    end
    return out
end
function M.loopclosure(n)
    local fs = {}
    for i = 1, n do
        fs[i] = function() return i end
    end
    return fs
end
function M.loopclosure2(t)
    local fs = {}
    for k, v in pairs(t) do
        local x = v * 2
        table.insert(fs, function() x += 1 return x + k end)
    end
    return fs
end
function M.whilecond(t) while t.n and t.n > 0 do t.n -= 1 end return t end
function M.whilecall(f) local c = 0 while f() do c += 1 end return c end
function M.earlyall(t, n)
    for i = 1, n do
        if t[i] then
            if t[i] > 5 then return "big" end
        else
            break
        end
    end
    return "done"
end
function M.lastbreak(t) for _, v in t do print(v) break end end
function M.lastreturn(t) for _, v in t do return v end end
function M.emptyfor(t) for _ in t do end end
function M.emptywhile(f) while f() do end end
function M.breakfirst(a, f, n)
    local i = 0
    repeat
        i += 1
        if not a or f(i) and not n then break end
    until i >= 3
    return i
end
function M.breakwhile(a, n)
    local i = 0
    while true do
        if not a then break end
        i += 1
        if i >= n then break end
    end
    return i
end
function M.breakelse(a, b)
    local i = 0
    while i < 3 do
        i += 1
        if a then
            break
        else
            if b then print("b") end
        end
    end
    return i
end
function M.ifthenwhile(target, node, nexts)
    local found = false
    if target ~= nil then
        while node ~= nil do
            local at = node
            if at == target then
                return true
            end
            node = nexts[at + 1]
        end
    end
    return found
end
function M.ifthenrepeat(a, n)
    if a then
        repeat
            n -= 1
        until n <= 0
    end
    return n
end
function M.ifandwhile(a, b, n)
    if a and b then
        while n > 0 do
            n -= 1
            if n == 2 then break end
        end
    end
    return n
end
print(M.ifthenwhile(3, 1, {2, 3}), M.ifthenwhile(9, 1, {2}), M.ifthenwhile(nil, 1), M.ifthenrepeat(true, 2), M.ifthenrepeat(false, 2), M.ifandwhile(1, 1, 5), M.ifandwhile(1, nil, 5))
print(M.breakfirst(false), M.breakfirst(true, function() return false end), M.breakfirst(true, function() return true end), M.breakwhile(false, 1), M.breakwhile(true, 2), M.breakelse(true), M.breakelse(false, true))
print(M.num(3), M.numstep(3), M.numstep2(4), M.numvar(1, 5, 2), M.pairsloop({1, 2}), M.ipairsloop({3, 4}), M.iter({5}), M.nextloop({6}), M.keyonly({1, 2}))
local function it() local d = false return function() if d then return nil end d = true return 1, 2, 3 end end
print(M.custom(it), M.whileloop(2), M.whiletrue(3), M.repeatloop(2), M.repeatlocal(3), M.repeatand(1, 2), M.repeator(1, 2))
print(M.brk({1, 3}), M.brk({1}), M.brk2({3}), M.cont({1, 2, 3}), M.contelse({1, 2, 3}), M.contwhile(5), M.contrepeat(3), M.nest(4), M.nestwhile(4))
print(#M.forwhile({2, 3}), M.loopclosure(2)[2](), M.loopclosure2({5})[1](), M.whilecond({n = 2}).n)
local cnt = 0 print(M.whilecall(function() cnt += 1 return cnt < 3 end), M.earlyall({1, 9}, 2), M.earlyall({1}, 2))
M.lastbreak({1, 2}) print(M.lastreturn({7})) M.emptyfor({}) M.emptywhile(function() end)
