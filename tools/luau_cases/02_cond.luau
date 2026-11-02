local M = {}
function M.guard(a) if not a then return nil end return a + 1 end
function M.guard2(a, b) if a == nil or b == nil then return 0 end return a + b end
function M.ifelse(a) if a then print("y") else print("n") end end
function M.chain(a)
    if a == 1 then return "a" elseif a == 2 then return "b" elseif a == 3 then return "c" else return "d" end
end
function M.chainassign(a)
    local r
    if a < 0 then r = "neg" elseif a == 0 then r = "zero" else r = "pos" end
    return r
end
function M.andcond(a, b) if a and b then return 1 end return 2 end
function M.orcond(a, b) if a or b then return 1 end return 2 end
function M.mixcond(a, b, c) if a and (b or c) then return 1 end return 2 end
function M.mixcond2(a, b, c) if (a or b) and c then return 1 end return 2 end
function M.mixcond3(a, b, c, d) if (a and b) or (c and d) then return 1 end return 2 end
function M.notcond(a, b) if not (a and b) then return 1 end return 2 end
function M.notcond2(a, b) if not (a or b) then return 1 end return 2 end
function M.cmpcond(a, b) if a > 0 and b < 10 or a == b then return 1 end return 2 end
function M.nested(a, b)
    if a then
        if b then return 1 else return 2 end
    else
        if b then return 3 end
    end
    return 4
end
function M.fallthrough(a)
    local x = 0
    if a > 1 then x = x + 1 end
    if a > 2 then x = x + 2 end
    if a > 3 then x = x + 4 end
    return x
end
function M.elseafter(a)
    if a then
        print("a")
    else
        if a == nil then print("nil") else print("false") end
        print("after")
    end
end
function M.emptythen(a) if a then else print("no") end end
function M.emptyelse(a) if a then print("yes") else end end
function M.emptyboth(f) if f() then end return 1 end
function M.typeguard(v)
    if type(v) ~= "table" then return tostring(v) end
    if v.kind == "a" and v.value ~= nil then return v.value end
    return nil
end
function M.multi(a, b)
    if a == "x" then
        local t = {}
        t.b = b
        return t
    elseif a == "y" and b then
        return b, b
    end
end
function M.condvar(a, b)
    local ok = a ~= nil and b ~= nil
    if ok and a > b then return a end
    return ok
end
function M.notnil(a) if a ~= nil then return a end return false end
function M.boolcmp(a) if a == true then return 1 elseif a == false then return 2 end return 3 end
function M.strcmp(a) if a == "x" or a == "y" or a == "z" then return 1 end return 0 end
function M.numcmp(a) if a >= 1 and a <= 10 then return "in" end return "out" end
function M.rangeflip(a) if 1 <= a and 10 >= a then return "in" end return "out" end
function M.callcond(f, a) if f(a) then return f(a + 1) end return f(a - 1) end
function M.deep(a, b, c)
    if a then
        if b then
            if c then return 3 end
            return 2
        end
        return 1
    end
    return 0
end
function M.returnor(a) if a then return a or 1 end return nil end
function M.assignand(a) local x = a and a.b if x then return x.c end end
function M.setinbranch(t, a) if a then t.x = 1 else t.y = 2 end t.z = 3 return t end
print(M.guard(1), M.guard(nil), M.guard2(1, 2), M.guard2(nil, 1)) M.ifelse(true) M.ifelse(false)
print(M.chain(1), M.chain(3), M.chain(9), M.chainassign(-1), M.chainassign(0), M.chainassign(1))
print(M.andcond(1, 2), M.orcond(nil, nil), M.mixcond(1, nil, 2), M.mixcond2(nil, 1, 2), M.mixcond3(nil, 1, 2, 3))
print(M.notcond(1, 2), M.notcond2(nil, nil), M.cmpcond(1, 2), M.cmpcond(-1, -1), M.nested(1, 1), M.nested(nil, 1), M.nested(nil, nil))
print(M.fallthrough(2), M.fallthrough(5)) M.elseafter(1) M.elseafter(nil) M.elseafter(false) M.emptythen(nil) M.emptyelse(1)
print(M.emptyboth(function() end), M.typeguard(1), M.typeguard({kind = "a", value = 2}), M.typeguard({}))
print(M.multi("x", 1).b, M.multi("y", 2)) print(M.multi("z", 1)) print(M.condvar(2, 1), M.condvar(nil, 1), M.notnil(1), M.notnil(nil))
print(M.boolcmp(true), M.boolcmp(false), M.boolcmp(1), M.strcmp("y"), M.strcmp("q"), M.numcmp(5), M.numcmp(11), M.rangeflip(3))
print(M.callcond(function(x) return x end, 2), M.deep(1, 1, 1), M.deep(1, 1), M.deep(1), M.deep(), M.returnor(1), M.returnor(nil))
print(M.assignand({b = {c = 1}}), M.assignand(nil), M.setinbranch({}, true).z)
