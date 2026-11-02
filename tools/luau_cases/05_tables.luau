local M = {}
function M.empty() return {} end
function M.list() return {1, 2, 3} end
function M.liststr() return {"a", "b"} end
function M.hash() return {a = 1, b = 2} end
function M.mixed() return {1, a = 2, 3} end
function M.computed(k) return {[k] = 1, [k .. "x"] = 2} end
function M.numkeys() return {[1] = "a", [2] = "b", [10] = "c"} end
function M.vals(a, b) return {a, b, a + b} end
function M.hvals(a, b) return {x = a, y = b, z = a + b} end
function M.dup(a) return {x = a, y = 1, z = "s"} end
function M.allconst() return {x = 1, y = 2, z = "s", w = true} end
function M.nested(a) return {p = {q = {r = a}}, l = {1, {2, {3}}}} end
function M.callend(f) return {1, f()} end
function M.callmid(f) return {f(), 1} end
function M.varend(...) return {1, ...} end
function M.nilvalue() return {a = nil, b = 1} end
function M.boolvalue() return {on = true, off = false} end
function M.longlist()
    return {1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36}
end
function M.fieldset() local t = {} t.a = 1 t.b = 2 return t end
function M.fieldthen() local t = {a = 1} t.b = 2 t[1] = 3 return t end
function M.idxset(t, k, v) t[k] = v t[1] = v t.x = v return t end
function M.rawkeys(t) t["a b"] = 1 t["ok"] = 2 return t end
function M.insert(t, v) table.insert(t, v) table.insert(t, 1, v) return table.remove(t) end
function M.nestedset(t) t.a.b.c = 1 t.a.b.d = t.a.b.c return t end
function M.compound(t, k) t.x += 1 t[k] -= 2 t.a.b *= 3 t[k .. "s"] ..= "z" return t end
function M.swap(t) t[1], t[2] = t[2], t[1] return t end
function M.fromcall(f) local a, b = f() return {a = a, b = b} end
function M.tablecall(f, a) f({x = a}) f({a}, {b = a}) return f{} end
function M.strcall(f) return f"x", f[[y]] end
function M.len(t) return #t + #t.a end
function M.concatlist(t) return table.concat(t, ",") end
function M.setmeta(t) return setmetatable({}, {__index = t}) end
function M.frozen() return table.freeze({a = 1}) end
function M.unpack(t) return table.unpack(t) end
function M.unpackmid(t) return table.unpack(t), 1 end
function M.typed() local p: {x: number} = {x = 1} return (p :: any).x end
function M.multiline(f, a)
    return {
        f(1),
        a + 1,
        a * 2,
        f(2),
    }
end
function M.multikeys(f, a)
    local t = {
        x = f(1),
        y = a + 1,
        [a] = f(2),
    }
    return t, t
end
function M.multicall(f, a)
    return f(
        a + 1,
        a * 2,
        f(a)
    )
end
function M.multiop(a, b)
    return a.x
        + b.y
        * a.z
end
function M.multiand(a, b)
    return a.x
        and b.y
        or a.z
end
print(#M.multiline(function(x) return x end, 1), M.multikeys(function(x) return x end, 1).y, M.multicall(function(a, b, c) return (a or 0) + (b or 0) + (c or 0) end, 1), M.multiop({x = 1, z = 2}, {y = 3}), M.multiand({x = 1}, {y = 2}), M.multiand({z = 3}, {}))
print(#M.empty(), #M.list(), #M.liststr(), M.hash().b, M.mixed()[2], M.computed("k").kx, M.numkeys()[10], #M.vals(1, 2), M.hvals(1, 2).z)
print(M.dup(1).x, M.allconst().w, M.nested(1).p.q.r, #M.callend(function() return 1, 2 end), #M.callmid(function() return 1, 2 end), #M.varend(1, 2))
print(M.nilvalue().b, M.boolvalue().off, #M.longlist(), M.fieldset().b, M.fieldthen()[1], M.idxset({}, "k", 1).x, M.rawkeys({})["a b"])
print(M.insert({1}, 2), M.nestedset({a = {b = {}}}).a.b.d) local c = M.compound({x = 1, k = 2, a = {b = 2}, ks = "a"}, "k") print(c.x, c.k, c.a.b, c.ks)
print(M.swap({1, 2})[1], M.fromcall(function() return 1, 2 end).b, M.tablecall(function(a) return a end, 1) ~= nil, M.strcall(function(s) return s end))
print(M.len({1, a = {2}}), M.concatlist({1, 2}), M.setmeta({a = 1}).a, M.frozen().a, M.unpack({1, 2})) print(M.unpackmid({1, 2})) print(M.typed())
