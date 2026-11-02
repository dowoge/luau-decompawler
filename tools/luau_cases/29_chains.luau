-- chains of tests that should come back as one condition
local M = {}
function M.typed(kind, v, r, f)
    if kind == "number" and type(v) == "number" then
        r.bar = f(v)
    elseif kind == "colour" and type(v) == "string" and #v == 6 then
        r.swatch = v
    end
    return r
end
function M.orurgent(d, gap, ur, urgent)
    if d >= gap or (d >= ur and urgent()) then
        print("flush")
    else
        print("tell")
    end
end
function M.twoword(can, x, parent)
    local label
    if can or parent(x.page_id) ~= nil then
        label = "Back"
    else
        label = "Close"
    end
    return label
end
function M.multilinecall(can, x, parent)
    local label
    if can or parent(
        x.page_id
    ) ~= nil then
        label = "Back"
    else
        label = "Close"
    end
    return label
end
function M.nestedcontinue(list, on, inner)
    local n = 0
    for _, v in list do
        if inner then
            if on and v.shown then
                local d = v.x * 2
                if d > 3 then
                    n += 100
                    continue
                end
            end
            n += 1
        end
        n += 2
    end
    return n
end
function M.twochecks(list, a)
    local n = 0
    for _, v in list do
        if v.t > 0 then
            local e = v.t - a
            if e > 1 then
                if v.flag then
                    v.flag = false
                    n += 1
                end
                if e > 2 then
                    n += 10
                    continue
                end
            end
        end
        if v.flag then
            n += 100
        end
    end
    return n
end
local r = M.typed("number", 1, {}, function(x) return x + 1 end) print(r.bar, M.typed("colour", "abcdef", {}).swatch, M.typed("x", 1, {}).bar)
M.orurgent(2, 1) M.orurgent(0, 1, 0, function() return true end) M.orurgent(0, 1, 0, function() return false end) M.orurgent(0, 1, 5)
local p = function(x) return x end print(M.twoword(true), M.twoword(false, {page_id = 1}, p), M.twoword(false, {}, p), M.multilinecall(true), M.multilinecall(false, {page_id = 1}, p), M.multilinecall(false, {}, p))
print(M.nestedcontinue({{shown = true, x = 3}, {shown = true, x = 1}, {x = 9}}, true, true), M.nestedcontinue({{}}, false, false), M.twochecks({{t = 5, flag = true}, {t = 3, flag = true}, {t = 0, flag = true}, {t = 2}}, 1))
-- a jump into the arm after one that leaves
local function hoisted(a, b, c)
    local x = 0
    if a then
        if b then
            x = 1
        else
            return "left"
        end
    end
    if c then
        x += 2
    end
    return x
end
print(hoisted(true, true, true), hoisted(true, false, true), hoisted(false, false, false))
-- `else continue`: its jump is threaded to the loop's end, leaving a dead one
local function kinds(records, parts)
    local n = 0
    for _, r in records do
        local p
        if r.tag == "Clip" then
            p = r.data
        elseif r.tag == "Pose" then
            p = r.data + 1
        else
            continue
        end
        if p ~= parts then
            n += p
        end
    end
    return n
end
local function kinds2(records, flag)
    local n = 0
    for i, r in records do
        local v, w
        if r.a == "x" and r.b then
            v = r.b
            w = r
        elseif r.a == "y" or r.a == "z" then
            v = r.a
            w = r
        elseif r.a == "q" then
            v = nil
            w = r
        else
            continue
        end
        n += i
        if w and v then
            n += 10
        end
    end
    return n
end
print(kinds({{tag = "Clip", data = 1}, {tag = "Pose", data = 2}, {tag = "No"}}, 3))
print(kinds2({{a = "x", b = 1}, {a = "y"}, {a = "q"}, {a = "n"}}))
