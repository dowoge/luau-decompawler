-- control flow that is hard to put back together
local M = {}
function M.statemachine(input)
    local state, i, out = "start", 1, {}
    while i <= #input do
        local c = input:sub(i, i)
        if state == "start" then
            if c == "a" then
                state = "seen_a"
            elseif c == "b" then
                table.insert(out, "B")
            end
        elseif state == "seen_a" then
            if c == "b" then
                table.insert(out, "AB")
                state = "start"
            elseif c ~= "a" then
                state = "start"
            end
        end
        i += 1
    end
    return table.concat(out, ",")
end
function M.nestedbreak(t)
    local found
    for _, row in ipairs(t) do
        for _, v in ipairs(row) do
            if v < 0 then
                found = v
                break
            end
        end
        if found then break end
    end
    return found
end
function M.continueinner(t)
    local n = 0
    for _, row in ipairs(t) do
        if #row == 0 then continue end
        for _, v in ipairs(row) do
            if v == 0 then continue end
            if v < 0 then break end
            n += v
        end
        n += 100
    end
    return n
end
function M.loopifret(t, k)
    for i, v in ipairs(t) do
        if v == k then
            if i == 1 then
                return "first"
            elseif i == #t then
                return "last"
            end
            return "mid"
        end
    end
    return "none"
end
function M.condinloop(n)
    local i = 0
    while true do
        i += 1
        if i > n then
            break
        elseif i % 2 == 0 then
            continue
        end
        print(i)
    end
    return i
end
function M.ifbreakelse(t)
    local r = 0
    for _, v in t do
        if v > 0 then
            r += v
        else
            if v == 0 then break end
            r -= v
        end
    end
    return r
end
function M.repeatbreak(n)
    local i = 0
    repeat
        i += 1
        if i == n then break end
    until i > 10
    return i
end
function M.whileretelse(t)
    local i = 1
    while t[i] do
        if t[i] == 0 then
            return i
        else
            i += 1
        end
    end
    return -1
end
function M.earlyexit(a, b, c)
    if a then
        print("a")
        if b then
            print("b")
            if c then
                print("c")
                return 3
            end
            print("not c")
        else
            print("not b")
        end
        print("end a")
    end
    return 0
end
function M.condcall(a, f) if a and f(a) or not a and f(1) then return 1 else return 2 end end
function M.shortvalue(a, b, c) local x = a and b or c or 5 return x end
function M.shortmix(a, b, c, d) local x = (a or b) and (c or d) return x end
function M.shortcmp(a, b) local x = a > 0 and a or b > 0 and b or 0 return x end
function M.shortnot(a, b) local x = not a or not b return x end
function M.shortarg(f, a, b) return f(a and b, a or b, not a) end
function M.shortconcat(a, b) return (a or "x") .. (b and "y" or "z") end
function M.shorttable(a, b) return {x = a or 1, y = b and 2, z = a and b or 3} end
function M.shortidx(t, k) return (t or {})[k], (t and t.a or {}).b end
function M.condassign(t, a) t.x = a and 1 or 2 t.y = not a return t end
function M.nestedifexpr(a, b) return (if a then 1 else 2) + (if b then 3 else 4) end
function M.ifexprcond(a, b) if (if a then b else not b) then return 1 end return 2 end
function M.ifexprcall(f, a) return f(if a then 1 else 2, if a then 3 else 4) end
function M.manyelseif(a)
    if a == 1 then print(1)
    elseif a == 2 then print(2)
    elseif a == 3 then print(3)
    elseif a == 4 then print(4)
    elseif a == 5 then print(5)
    elseif a == 6 then print(6)
    else print(0) end
end
function M.elseifcompound(a, b)
    if a and b then return 1
    elseif a or b then return 2
    elseif not a and not b then return 3 end
end
function M.dispatch(op, a, b)
    local r
    if op == "+" then r = a + b
    elseif op == "-" then r = a - b
    elseif op == "*" then r = a * b
    elseif op == "/" then
        if b == 0 then return nil, "div0" end
        r = a / b
    else
        return nil, "bad"
    end
    return r
end
print(M.statemachine("abxab"), M.nestedbreak({{1}, {2, -1}}), M.nestedbreak({{1}}), M.continueinner({{1, 0, 2}, {}, {-1, 3}}), M.loopifret({1, 2, 3}, 1), M.loopifret({1, 2, 3}, 2), M.loopifret({1, 2, 3}, 3), M.loopifret({}, 1))
print(M.condinloop(3), M.ifbreakelse({1, -2, 0, 5}), M.repeatbreak(3), M.repeatbreak(20), M.whileretelse({1, 0}), M.whileretelse({1}))
print(M.earlyexit(1, 1, 1), M.earlyexit(1, 1), M.earlyexit(1), M.earlyexit()) print(M.condcall(1, function(x) return x end), M.condcall(nil, function(x) return nil end), M.shortvalue(1, 2, 3), M.shortvalue(nil, 2, nil), M.shortmix(nil, 1, nil, 2), M.shortcmp(0, 1), M.shortcmp(0, 0), M.shortnot(1, 2))
print(M.shortarg(function(...) return ... end, 1, 2)) print(M.shortconcat(nil, 1), M.shorttable(1, nil).z, M.shortidx({a = {b = 1}, k = 2}, "k")) print(M.condassign({}, 1).x, M.nestedifexpr(1, nil), M.ifexprcond(1, 2), M.ifexprcall(function(a, b) return a + b end, true))
M.manyelseif(3) M.manyelseif(9) print(M.elseifcompound(1, 1), M.elseifcompound(1), M.elseifcompound(), M.dispatch("+", 1, 2), M.dispatch("/", 1, 0)) print(M.dispatch("?", 1, 2))
