-- How many values a call yields depends on where it stands and on what its function
-- provably returns: spread last, cut to one by a parameter, a local or parentheses.
function Ranks()
	return "gold", "silver", "bronze"
end
local function headline(first)
	return "top " .. tostring(first)
end
local function announceAll()
	print("ranks", Ranks())
end
local function announceFirst()
	print("ranks", (Ranks()))
end
announceAll()
announceFirst()
print(headline(Ranks()))

function Pair(tag)
	return tag, #tag
end
local function forwarder()
	return function()
		return Pair("fw")
	end
end
local forward = forwarder()
local function firstOnly()
	local tag = Pair("one")
	return tag
end
print(select("#", firstOnly()), firstOnly(), select("#", forward()), forward())

local function xpCurve(level)
	local xp = 50
	for _ = 2, level do
		xp = xp * 2
		if xp > 1e7 then
			return -1
		end
	end
	local text = tostring(xp)
	if #text > 3 then
		return tonumber(text:sub(1, 3)) * 10 ^ (#text - 3)
	end
	return xp
end
local function splitStack(n)
	if n > 64 then
		return n - 64, 64
	end
	return splitStack(n * 3)
end
local levels = { xpCurve(3), xpCurve(9) }
print(xpCurve(2), select("#", xpCurve(4)), xpCurve(20))
print(#levels, levels[2], select("#", splitStack(5)), splitStack(5))
print((math.max(xpCurve(5), 1)), math.max(xpCurve(1), 1))

local function flip(v)
	return not v
end
local function label(flag)
	if flag then
		return 1
	end
	return "one"
end
local function triple()
	return 0, "mid", nil
end
local function onlyIf(flag)
	if flag then
		return "yes"
	end
end
local function nothingOrNil(flag)
	if flag then
		return nil
	end
	return
end
local function addOne(v: number)
	return v + 1
end
local function rounded()
	return math.floor(9.5)
end
local mathBackup = math
math = {
	floor = function()
		return "replaced"
	end,
}
local odd = setmetatable({}, {
	__add = function()
		return "meta"
	end,
})
for _, flag in { false, true } do
	print(flip(flag), label(flag), onlyIf(flag))
	print("count", select("#", nothingOrNil(flag)), select("#", onlyIf(flag)))
end
print("triple", select("#", triple()), triple())
print("runtime", addOne(odd), rounded())
math = mathBackup
local Shop = {}
print("missing", Shop.buy)
function Shop:buy(item)
	return item == self
end
print("method", Shop:buy(Shop))
local function negateInner()
	local inner = function(v)
		return not v
	end
	return inner(0)
end
print("inner", negateInner())
return flip, label, triple, onlyIf, nothingOrNil, addOne, rounded
