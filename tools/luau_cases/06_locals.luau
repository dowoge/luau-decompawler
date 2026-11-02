local M = {}
function M.unused() local a = 1 local b = 2 return 3 end
function M.unusedcall(f) local a = f() local b = f() return 1 end
function M.nodecl() local a return a end
function M.multinil() local a, b, c return a, b, c end
function M.multi() local a, b = 1, 2 return b, a end
function M.fewer() local a, b = 1 return a, b end
function M.more(f) local a = f(), 2 return a end
function M.fromcall(f) local a, b, c = f() return c, b, a end
function M.callmix(f) local a, b, c = 1, f() return a, b, c end
function M.shadow(a) local a = a + 1 local a = a * 2 return a end
function M.shadowblock(a) local x = a do local x = 2 print(x) end return x end
function M.doempty() do end return 1 end
function M.doscope() do local a = 1 print(a) end do local b = 2 print(b) end end
function M.reuse()
    do local a = 1 print(a) end
    local b = 2
    print(b)
end
function M.swap(a, b) a, b = b, a return a, b end
function M.rotate(a, b, c) a, b, c = b, c, a return a, b, c end
function M.reassign(a) a = a + 1 a = a * 2 return a end
function M.assigncopy(a) local b = a a = 2 return a, b end
function M.copythen(a) local b = a b = b + 1 return a, b end
function M.chainassign(t) local a = t.x local b = a.y local c = b.z return c end
function M.keep(t) local a = t.x return a, a end
function M.keepcall(f) local a = f() return a + a end
function M.orderkeep(f, g) local a = f() local b = g() return b, a end
function M.orderinline(f, g) local a = f() return g(), a end
function M.sideeffect(t) local a = t.x t.x = 2 return a end
function M.argorder(f, g, h) return f(g(), h()) end
function M.branchvar(c)
    local x = 1
    if c then
        x = 2
    end
    return x
end
function M.branchvar2(c, d)
    local x
    local y = 0
    if c then
        x = 2
        y = 1
    elseif d then
        x = 3
    end
    return x, y
end
function M.ifretain(c)
    local v = c and 1 or 2
    local w = c and "a"
    return v, w
end
function M.loopdecl(n)
    local total = 0
    for i = 1, n do
        local a = i * 2
        local b = a + 1
        total += b
    end
    return total
end
function M.assignfield(t, v) local old = t.v t.v = v return old end
function M.multiassign(t) t.a, t.b = 1, 2 return t end
function M.multiassigncall(t, f) t.a, t.b = f() return t end
function M.mixassign(t) local a a, t.x = 1, 2 return a, t end
function M.tempexpr(a) return (a + 1) * (a + 1) end
function M.constlocal() local K = 5 return K * 2 end
function M.conststr() local S = "x" return S .. "y" end
function M.lateuse(f) local a = 1 f() f() return a end
function M.lots()
    local a, b, c, d, e, f, g, h = 1, 2, 3, 4, 5, 6, 7, 8
    return h, g, f, e, d, c, b, a
end
print(M.unused(), M.unusedcall(print), M.nodecl(), M.multinil()) print(M.multi()) print(M.fewer()) print(M.more(function() return 1 end))
print(M.fromcall(function() return 1, 2, 3 end)) print(M.callmix(function() return 1, 2 end)) print(M.shadow(1), M.shadowblock(1), M.doempty()) M.doscope() M.reuse()
print(M.swap(1, 2)) print(M.rotate(1, 2, 3)) print(M.reassign(1), M.assigncopy(1)) print(M.copythen(1)) print(M.chainassign({x = {y = {z = 1}}}), M.keep({x = 1}))
print(M.keepcall(function() return 2 end), M.orderkeep(function() return 1 end, function() return 2 end)) print(M.orderinline(function() return 1 end, function() return 2 end))
print(M.sideeffect({x = 1}), M.argorder(function(a, b) return a + b end, function() return 1 end, function() return 2 end), M.branchvar(true), M.branchvar(false)) print(M.branchvar2(false, true)) print(M.ifretain(true)) print(M.ifretain(false))
print(M.loopdecl(3), M.assignfield({v = 1}, 2), M.multiassign({}).b, M.multiassigncall({}, function() return 1, 2 end).b) print((M.mixassign({}))) print(M.tempexpr(1), M.constlocal(), M.conststr(), M.lateuse(print)) print(M.lots())
