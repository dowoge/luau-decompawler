-- Helpers returning a fixed set of values, copied where callers take each one: a local
-- shared with a result, a copied helper inside, a captured counter and a spread tail.
local function per(a, b)
	if b == 0 then
		return 0
	end
	return a / b
end

local function rates(stats)
	local hit = per(stats.hits, stats.shots)
	return hit, per(stats.kills, stats.deaths)
end

local function band(score)
	local tier = score >= 50 and "gold" or "iron"
	return tier, tier .. score
end

local function grow(stock)
	local before = stock.count
	stock.count = before * 3
	return before, before + stock.count
end

local function ticker(from)
	local value = from
	local function step()
		value -= 1
		return value
	end
	return step, value
end

local function firstComma(s)
	local len = #s
	return len, string.find(s, ",", 1, true)
end

local R = {}
function R.summary(stats)
	local hit, kd = rates(stats)
	return string.format("%.2f/%.2f", hit, kd)
end
function R.withRatio(tag, a, b)
	return tag, per(a, b)
end
function R.tier(score)
	local tier, label = band(score)
	return tier .. "|" .. label
end
function R.restock(stock)
	local before, total = grow(stock)
	return before * 100 + total
end
function R.countdown()
	local step, initial = ticker(10)
	step()
	return step() + initial, initial
end
function R.comma(s)
	local len, at = firstComma(s)
	return len + (at or -1)
end

local tag, ratio = R.withRatio("acc", 3, 4)
print(R.summary({ hits = 7, shots = 10, kills = 5, deaths = 0 }), tag, ratio, R.tier(70), R.tier(20))
print(R.restock({ count = 4 }), R.countdown(), R.comma("a,b"), R.comma("ab"))

local function spread(values)
	local low, high = math.huge, -math.huge
	local lowAt = 0
	local flags = {}
	local sum = 0
	local text = ">"
	for i = 1, #values do
		local v = values[i]
		if v < low then
			low = v
			lowAt = i
		end
		if v > high then
			high = v
		end
		flags[i] = v > 0
		sum += v
		text ..= v
	end
	return low, high, lowAt, flags, sum, text
end
local low, high, at, flags, sum, text = spread({ 4, -2, 9 })
print(low, high, at, #flags, flags[2], sum, text)
local low2, high2, at2, flags2, sum2, text2 = spread({})
print(low2, high2, at2, #flags2, sum2, text2)
