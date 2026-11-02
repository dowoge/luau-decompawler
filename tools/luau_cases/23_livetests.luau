-- conditions on variables the body goes on to use
local M = {}
function M.andlocal(a, b)
    local v = a.x
    local w = a.y
    if v and w then
        return v + w
    else
        print("missing")
    end
end
function M.andlocal2(p, q)
    local s = p.s
    local t = q.t
    if s and t then
        print(s, t)
    else
        print("no")
        if p.warn then print("warn") end
    end
end
function M.threelocals(a, b, c)
    if a and b and c then
        return a + b + c
    end
    return 0
end
function M.orlocal(a, b)
    if a or b then
        print(a, b)
    else
        print("neither")
    end
end
function M.mixedlive(a, b, c)
    if a and (b or c) then
        print(a, b, c)
    else
        print("not", a, b, c)
    end
end
function M.notlocal(a, b)
    if not a or not b then
        print("missing", a, b)
        return
    end
    print(a + b)
end
function M.fieldtest(self, key)
    local speaker = self.speakers[key]
    local other = self.other
    if speaker and other then
        if speaker.muted then
            print("muted")
        elseif not other.ok then
            print("blocked")
        else
            print("send", speaker.name, other.name)
        end
    else
        if self.debug then
            print("not in channel", key)
        end
    end
end
function M.camera(self, torso, enabled)
    local cam = self.cam
    if self.follow and torso and torso.Part and cam then
        cam.pos = torso.Part.pos
        return cam
    end
    if enabled then print("idle") end
    return nil
end
function M.compare(a, b)
    if a > 0 and b then
        return b
    elseif a < 0 and b then
        return -b
    end
    return b
end
function M.isa(v)
    local x = v.child
    if x:IsA("Int") or x:IsA("Num") then
        return x.Value
    end
    return tostring(x)
end
function M.makeorkill(list, x)
    for _, c in list do
        local y = c:Find()
        if x and not y then
            print("make")
        elseif not x and y then
            y:Destroy()
        end
    end
end
function M.tween(open, current, make)
    if open and not current then
        current = make()
    elseif not open and current then
        current:Cancel()
    end
    return current
end
function M.preview(shop, featured, sel)
    local v = sel
    if v and not (if featured then shop.a else shop.b)[v] then
        print("stop")
    elseif v then
        print("draw", v)
    end
end
local FLAG_OFF = false
function M.andfalse(a, x)
    if a and FLAG_OFF then
        print("dead", x)
    else
        print("live", x)
    end
    return x
end
function M.boolpair(a, b, seen)
    if a == (seen.far == true) and b == (seen.off == true) then
        print("same")
    else
        seen.far = a
        print("changed")
    end
end
function M.orintest(t, a, rest)
    if a and t - (rest or 5) >= 0 then
        print("ready")
    elseif t < 0 then
        print("early")
    end
end
function M.ifexprtest(v, t, on)
    if v and not t[if on then "a" else "b"] then
        print("stop")
    elseif v then
        print("draw")
    end
end
function M.earlycontinue(list, on)
    local n = 0
    for _, v in list do
        if on and v.shown then
            local d = v.x * 2
            if d > 3 then
                n += 100
                continue
            end
        end
        n += 1
    end
    return n
end
print(M.andfalse(1, 2)) M.boolpair(true, true, {far = true, off = true}) M.boolpair(true, false, {far = true}) M.orintest(9, 1) M.orintest(1, 1) M.orintest(-1) M.ifexprtest(1, {a = 1}, true) M.ifexprtest(1, {}, true) M.ifexprtest()
print(M.earlycontinue({{shown = true, x = 3}, {shown = true, x = 1}, {x = 9}}, true), M.earlycontinue({{x = 1}}))
M.makeorkill({{Find = function() return nil end}}, true) M.makeorkill({{Find = function() return {Destroy = function() print("gone") end} end}}, false)
print(M.tween(true, nil, function() return 1 end), M.tween(false, nil), M.tween(true, 2)) M.preview({a = {}}, true, 1) M.preview({b = {true}}, false, 1) M.preview({a = {}}, true)
print(M.andlocal({x = 1, y = 2}), M.andlocal({x = 1})) M.andlocal2({s = 1}, {t = 2}) M.andlocal2({warn = true}, {}) print(M.threelocals(1, 2, 3), M.threelocals(1, nil, 3)) M.orlocal(1) M.orlocal() M.mixedlive(1, 1) M.mixedlive(1) M.mixedlive() M.notlocal(1) M.notlocal(1, 2)
M.fieldtest({speakers = {k = {muted = true}}, other = {}}, "k") M.fieldtest({speakers = {k = {name = "s"}}, other = {}}, "k") M.fieldtest({speakers = {k = {name = "s"}}, other = {ok = true, name = "o"}}, "k") M.fieldtest({speakers = {}, debug = true}, "k") M.fieldtest({speakers = {}}, "k")
print(M.camera({follow = true, cam = {}}, {Part = {pos = 1}}).pos, M.camera({}, nil, true), M.compare(1, 2), M.compare(-1, 2), M.compare(0, 2))
local mk = function(ok) return {child = {IsA = function(_, n) return ok == n end, Value = 7}} end print(M.isa(mk("Int")), M.isa(mk("Num")), M.isa(mk("x")) ~= nil)
