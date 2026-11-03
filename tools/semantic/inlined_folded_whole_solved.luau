-- Helpers given numbers whose copies fold to one constant: `frames(13)` is
-- the number 13 / 60 in the bytecode, and `toRad(90)` is pi / 2. The calls
-- come back with arguments that make those very numbers.
local function frames(n)
    return n / 60
end
local function toRad(d)
    return d * math.pi / 180
end
local function half(x)
    return x * 0.5 + 1
end
print(frames(13), frames(90), toRad(90), toRad(45), half(7))
local t = { frames(3), toRad(180) }
print(t[1], t[2], half(-3) + frames(30))
