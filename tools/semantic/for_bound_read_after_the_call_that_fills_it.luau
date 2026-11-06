-- A for loop's start and bound, both `cache.k or (function(...) ... end)(...)`
-- in its header, all on one line as minified code has it: the start's call
-- also fills the bound's key. The bound comes out as a local, read and then
-- made if missing; that read was written out above the start's call, so it
-- saw the cache before the call had filled it.
local cache = {}
local calls = 0

local function step(a, b)
    calls += 1
    return a + b
end

local function body(t)
    local made = {}
    for i = cache.start or (function(...) local args = { ... } cache.start = t[1](args[1][1], 1) cache.bound = args[2][1] return cache.start end)({ 2 }, { 4 }), t[2] - (cache.bound or (function(...) local args = { ... } cache.bound = t[1](args[1][1], args[2][1]) return cache.bound end)({ 5 }, { 6 })) do
        made[#made + 1] = i
    end
    return #made, cache.start, cache.bound
end

print(body({ step, 20 }))
cache = {}
print(body({ step, 30 }))
cache = { start = 2 }
print(body({ step, 19 }))
print(calls)
