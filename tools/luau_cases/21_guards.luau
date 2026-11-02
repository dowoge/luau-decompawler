-- early returns inside ifs: no else here, whatever the jumps look like
local M = {}
function M.nestedguard(a, b)
    if a then
        if b then
            print("both")
            return
        end
    end
    print("rest")
end
function M.nestedguard2(a, b, c)
    if a then
        if b then
            if c then
                print("all")
                return
            end
        end
    end
    print("rest")
    if a then print("a") end
end
function M.twoguards(a, b)
    if a then
        print("a")
        return
    end
    if b then
        print("b")
        return
    end
    print("neither")
end
function M.mixedguard(a, b)
    if a then
        if b then
            print("x")
            return
        end
        print("y")
    else
        print("z")
    end
    print("rest")
end
function M.loopguard(t)
    for _, v in t do
        if v > 0 then
            if v > 10 then
                print("big")
                return
            end
        end
    end
    print("done")
end
function M.chain(a, b, c)
    if not a then return end
    if not b then return end
    if not c then
        print("no c")
        return
    end
    print("all")
end
function M.andguard(a, b)
    if a and b then
        print("ab")
        return
    end
    print("rest")
    if b then print("b") end
end
function M.orguard(a, b)
    if a or b then
        print("a or b")
        return
    end
    print("rest")
    if a then print("never") end
end
function M.valueguard(a, b)
    if a then
        if b then return 1, 2 end
    end
    return 3
end
function M.manyvalues(h, r, p)
    if h and not (h.dead) then
        if r and r:IsPart() then
            if p then
                if p:IsPart() then
                    return h, r, p
                end
            end
        end
    end
    return nil, nil, nil
end
M.nestedguard(1, 1) M.nestedguard(1) M.nestedguard() M.nestedguard2(1, 1, 1) M.nestedguard2(1, 1) M.twoguards(1) M.twoguards(nil, 1) M.twoguards() M.mixedguard(1, 1) M.mixedguard(1) M.mixedguard()
M.loopguard({1, 11}) M.loopguard({1}) M.chain() M.chain(1) M.chain(1, 1) M.chain(1, 1, 1) M.andguard(1, 1) M.andguard(1) M.orguard(1) M.orguard() print(M.valueguard(1, 1)) print(M.valueguard(1)) print(M.valueguard())
local part = {IsPart = function() return true end} print(M.manyvalues({}, part, part) ~= nil, M.manyvalues({dead = true}, part, part), M.manyvalues({}, nil, part))
