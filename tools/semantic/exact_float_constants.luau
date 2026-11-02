-- Repeating-decimal constants written as decimals or divisions and used as operands,
-- keys and arguments: each must come back as the identical double.
local x = tonumber("6")
local function hex(v)
	return string.format("%.17g", v)
end

local named = {
	{ "5/7", 0.7142857142857143 },
	{ "13/45", 0.28888888888888886 },
	{ "1/90", 0.011111111111111112 },
	{ "47/36", 1.3055555555555556 },
	{ "-7/12", -0.5833333333333334 },
	{ "1/983", 0.0010172939979654121 },
	{ "4/9 by division", 4 / 9 },
	{ "22/7", 3.142857142857143 },
}
for _, pair in ipairs(named) do
	print(pair[1], hex(pair[2]))
end

local forms = {
	function() return x * 0.7142857142857143 end,
	function() return 0.7142857142857143 / x end,
	function() return 9 // 0.7142857142857143 end,
	function() return x % 0.14285714285714285 end,
	function() return 3 ^ 0.14285714285714285 end,
	function() return 0.1111111111111111 ^ x end,
	function() return x - -0.7142857142857143 end,
	function() return -(x > 1 and 0.1111111111111111 or 0) end,
}
local shown = {}
for i, f in ipairs(forms) do
	shown[i] = hex(f())
end
print(table.concat(shown, " "))
print(x / 9 < 0.7142857142857143, x / 8 < 0.7142857142857143, "q=" .. 0.28888888888888886)

local weights = { [0.14285714285714285] = "a seventh", ninth = 0.1111111111111111 }
print(weights[1 / 7], weights[0.14285714285714285], hex(weights.ninth), hex(math.min(0.08333333333333333, 0.5)))

local waits = {}
Clock = {}
function Clock.wait(s)
	waits[#waits + 1] = s
end
local function pauses()
	Clock.wait(0.08333333333333333)
	Clock.wait(0.08333333333333333)
	Clock.wait(0.08333333333333333)
end
pauses()
Clock.wait(1 / 12)
print(hex(waits[1] + waits[2] + waits[3]), waits[1] == waits[4], waits[2] == 1 / 12)

local plain = { 0.30000001192092896, 0.9970089730807578, 0.7142857142857144, 0.45 + 0.375, 7 + 0.5, 6.283185307179586 }
for i, v in ipairs(plain) do
	plain[i] = hex(v)
end
print(table.concat(plain, " "), 1 / -0, 1 / 0)
