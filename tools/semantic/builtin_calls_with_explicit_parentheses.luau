-- Builtins cut to one value with parentheses, called through local aliases,
-- keep their argument reads, errors and frames, even with every register taken.
local kindOf = type
local typeOfAlias = typeof

local probes = {
	function(v) return (kindOf(v)) end,
	function(v) return (typeOfAlias(v)) end,
	function(v) return (math.floor(v)) end,
	function(v) return (string.rep(v, 2)) end,
	function(v) return (bit32.bor(v, 8)) end,
}
local inputs = { "nil", true, -2.5, "ab", 5 }
inputs[1] = nil
for i, probe in ipairs(probes) do
	local ok, value = pcall(probe, inputs[i])
	print(i, ok, value)
end

local reads = 0
local watched = setmetatable({}, {
	__index = function(_, key)
		reads += 1
		print("index", key, reads)
		return 12
	end,
})
local function sample(obj)
	local value = obj.charge
	print("value", value)
	return (kindOf(value))
end
print("plain", pcall(sample, watched))

local function sampleLater(obj)
	local value = obj.charge
	local get = function()
		return value
	end
	local k = typeOfAlias(value)
	print("kind", k)
	return get
end
local g1, g2 = sampleLater(watched), sampleLater(watched)
print("same closure", g1 == g2, g1(), g2())

local failing = setmetatable({}, {
	__index = function(_, key)
		print("failing index", key)
		error("no charge", 0)
	end,
})
print("failing", pcall(sample, failing))

local inside = false
local nested = setmetatable({}, {
	__index = function(_, key)
		print("nested index", key, inside, debug.info(2, "f") == sample)
		if not inside then
			inside = true
			print("inner", pcall(sample, { charge = "x" }))
			inside = false
		end
		return 1
	end,
})
print("nested", pcall(sample, nested))

local function keyedRead(obj, key)
	local value = obj[key]
	print("keyed", value)
	return (kindOf(value))
end
print("dynamic", pcall(keyedRead, watched, "power"))

local function viaCall(produce)
	local value = produce()
	print("produced", value)
	return (kindOf(value))
end
print("call", pcall(viaCall, function()
	print("producing")
	return "s", 99
end))

local function viaVarargs(...)
	local value = ...
	print("vararg", value)
	return (kindOf(value))
end
print("none", pcall(viaVarargs))
print("one", pcall(viaVarargs, false))
print("many", pcall(viaVarargs, 3, "extra", nil))

local shared = 5
local function afterCallback(cb)
	cb()
	local value = shared
	print("shared", value)
	return (kindOf(value))
end
print("callback", pcall(afterCallback, function()
	shared = "str"
end))

local function crowded(...)
	local r0, r1, r2, r3, r4, r5, r6, r7, r8, r9, r10, r11, r12, r13, r14, r15, r16, r17, r18, r19, r20, r21, r22, r23, r24, r25, r26, r27, r28, r29, r30, r31, r32, r33, r34, r35, r36, r37, r38, r39, r40, r41, r42, r43, r44, r45, r46, r47, r48, r49, r50, r51, r52, r53, r54, r55, r56, r57, r58, r59, r60, r61, r62, r63, r64, r65, r66, r67, r68, r69, r70, r71, r72, r73, r74, r75, r76, r77, r78, r79, r80, r81, r82, r83, r84, r85, r86, r87, r88, r89, r90, r91, r92, r93, r94, r95, r96, r97, r98, r99, r100, r101, r102, r103, r104, r105, r106, r107, r108, r109, r110, r111, r112, r113, r114, r115, r116, r117, r118, r119, r120, r121, r122, r123, r124, r125, r126, r127, r128, r129, r130, r131, r132, r133, r134, r135, r136, r137, r138, r139, r140, r141, r142, r143, r144, r145, r146, r147, r148, r149, r150, r151, r152, r153, r154, r155, r156, r157, r158, r159, r160, r161, r162, r163, r164, r165, r166, r167, r168, r169 = ...
	local value = r1
	local observed = (kindOf)(value)
	print("crowded kind", observed, math.max(r0, r169))
	return { r0, r1, r2, r3, r4, r5, r6, r7, r8, r9, r10, r11, r12, r13, r14, r15, r16, r17, r18, r19, r20, r21, r22, r23, r24, r25, r26, r27, r28, r29, r30, r31, r32, r33, r34, r35, r36, r37, r38, r39, r40, r41, r42, r43, r44, r45, r46, r47, r48, r49, r50, r51, r52, r53, r54, r55, r56, r57, r58, r59, r60, r61, r62, r63, r64, r65, r66, r67, r68, r69, r70, r71, r72, r73, r74, r75, r76, r77, r78, r79, r80, r81, r82, r83, r84, r85, r86, r87, r88, r89, r90, r91, r92, r93, r94, r95, r96, r97, r98, r99, r100, r101, r102, r103, r104, r105, r106, r107, r108, r109, r110, r111, r112, r113, r114, r115, r116, r117, r118, r119, r120, r121, r122, r123, r124, r125, r126, r127, r128, r129, r130, r131, r132, r133, r134, r135, r136, r137, r138, r139, r140, r141, r142, r143, r144, r145, r146, r147, r148, r149, r150, r151, r152, r153, r154, r155, r156, r157, r158, r159, r160, r161, r162, r163, r164, r165, r166, r167, r168, r169 }
end
local packed = crowded(0, 3, 6, 9, 12, 15, 18, 21, 24, 27, 30, 33, 36, 39, 42, 45, 48, 51, 54, 57, 60, 63, 66, 69, 72, 75, 78, 81, 84, 87, 90, 93, 96, 99, 102, 105, 108, 111, 114, 117, 120, 123, 126, 129, 132, 135, 138, 141, 144, 147, 150, 153, 156, 159, 162, 165, 168, 171, 174, 177, 180, 183, 186, 189, 192, 195, 198, 201, 204, 207, 210, 213, 216, 219, 222, 225, 228, 231, 234, 237, 240, 243, 246, 249, 252, 255, 258, 261, 264, 267, 270, 273, 276, 279, 282, 285, 288, 291, 294, 297, 300, 303, 306, 309, 312, 315, 318, 321, 324, 327, 330, 333, 336, 339, 342, 345, 348, 351, 354, 357, 360, 363, 366, 369, 372, 375, 378, 381, 384, 387, 390, 393, 396, 399, 402, 405, 408, 411, 414, 417, 420, 423, 426, 429, 432, 435, 438, 441, 444, 447, 450, 453, 456, 459, 462, 465, 468, 471, 474, 477, 480, 483, 486, 489, 492, 495, 498, 501, 504, 507)
print("crowded", #packed, packed[2], packed[170])
print("reads", reads)
