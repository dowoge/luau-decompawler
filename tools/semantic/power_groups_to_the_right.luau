-- `^` groups to the right: (a ^ b) ^ c is not a ^ b ^ c
local a, b, c = tonumber("2"), tonumber("3"), tonumber("2")
local n = tonumber("-2")
print((a ^ b) ^ c, a ^ b ^ c, a ^ (b ^ c))
print(-a ^ 2, (-a) ^ 2, a ^ -b, -2 ^ a, (-2) ^ a, n ^ a)
print((2 ^ a) ^ b, 2 ^ a ^ b, (a ^ 2) ^ 0.5, (a * b) ^ c, a * b ^ c, a ^ (b - c))
local t = {v = a}
print((t.v ^ b) ^ (c ^ a), ((a ^ b) ^ c) ^ 0.5)
