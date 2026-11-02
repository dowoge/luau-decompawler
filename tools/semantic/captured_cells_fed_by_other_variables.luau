-- A captured local that receives another variable's value stays its own cell:
-- closure writes to it never reach the source, and two captured parameters never merge.
local function flip(p, q)
	return q, p
end

local function crossWrite(x, y)
	local a, b, c = x, y, nil
	c, a = flip(b, b)
	local getB
	for _ = 1, 2 do
		getB = function()
			return b
		end
	end
	local setA = function(z)
		a = z
		return a
	end
	return setA(99), a, getB(), b, c
end
print(crossWrite(1, 2))

local function crossMerge(x, y, flag)
	local a, b = x, y
	if flag then
		a = b
	end
	local history = {}
	for _ = 1, 2 do
		history[#history + 1] = tostring(a)
	end
	local setA = function(z)
		a = z
	end
	setA(7)
	return table.concat(history, " "), a, b
end
print(crossMerge(1, 2, true))
print(crossMerge(1, 2, false))

local ticks = 0
local function sample(seed)
	ticks += 1
	return ticks % 2 == 0 and ticks or nil, (seed or 0) + 10
end
local readers = {}
local function scan(seed, remember)
	local hit, level = sample(seed)
	if hit then
		readers[1] = function(z)
			level = z
			return level
		end
	else
		for round = 1, 3 do
			hit, level = sample(level)
			readers[2] = function()
				return level + round
			end
			if hit then
				break
			end
		end
		if remember then
			readers[3] = function()
				return hit
			end
		end
	end
	return hit, level
end
print(scan(1, true))
for i = 1, 3 do
	if readers[i] then
		print(i, readers[i](i))
	end
end

local stash
local function order(lo, hi)
	if lo <= 0 then
		stash = function()
			return hi
		end
	else
		local width = string.len(tostring(lo))
		if hi < width then
			hi, width = width, hi
		end
		lo += width
	end
	return lo, hi
end
print(order(42, 1))
print(order(42, 5))
print(order(-3, 8), stash())

local function clampRange(minV, maxV)
	local readMax
	if minV > maxV then
		minV, maxV = maxV, minV
	else
		readMax = function()
			return maxV
		end
	end
	local readMin = function()
		return minV
	end
	return minV, maxV, readMin(), readMax and readMax() or "none"
end
print(clampRange(9, 4))
print(clampRange(2, 6))
