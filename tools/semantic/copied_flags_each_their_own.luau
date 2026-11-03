-- A value copied into both arms of an `or` (`if a == nil or b then`), which
-- holds an and/or the decompiler first writes with a flag: each copy's flag
-- is its own, and each goes on its own.
local arr6 = {4, 5}
local obj7 = {x = 3}
function obj7:get()
    return self.x * 2
end
local function f1(c)
    if c then
        return 1
    end
    return 2
end
local function f4(x)
    return x
end
local cases = {
    {false, "{q}", true, 1, 2.5, false, 3, 2, true, 1},
    {1, "{q}", true, false, 2.5, true, 3, 2, true, -4},
    {1, "ab", false, 5, 1, false, 3, 2, false, 6},
    {1, "{q}", false, 7, 9, true, 1, 8, true, 0},
    {false, "zz", true, nil, 0.5, false, 2, 1, nil, 3},
}
for i117, case in cases do
    local v66, v53, v13, v116, v72, v54, v34, v96, v10, v40 = table.unpack(case, 1, 10)
    if v66 == false then
        v66 = nil
    end
    local t119 = {f4((if (v66 == nil or string.len(v53) <= (if v13 then 2.25 else v72)) then string.len(tostring(10)) - f1((math.abs(7)) % 3 - f1((((v10 or false) and v53 == "{q}") and v116 or math.abs(v40) + f1(true)) > (if v54 then obj7:get() else (if false then i117 else v34) - #arr6)) < -v72) else (math.max(math.min(5, v96), #arr6)) * 3)), select(1, (v116 or 1) * 3, obj7.x)}
    print(i117, #t119, t119[1], t119[2], t119[3])
end
