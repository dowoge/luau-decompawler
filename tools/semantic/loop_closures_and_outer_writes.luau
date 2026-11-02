-- Closures in and around loops share outer locals: writes after a loop reach the
-- closures made in it, and writes by closures (an iterator too) reach later reads.
local function lockButtons(count, locked)
	local list = {}
	for i = 1, count do
		list[i] = {
			IsLocked = function()
				return locked
			end,
		}
	end
	locked = not locked
	return list
end
print("locked", lockButtons(2, true)[2].IsLocked())

local function scaled(n)
	local readers = {}
	local i = 0
	while i < 3 do
		i += 1
		readers[i] = function()
			return n + i
		end
	end
	n = n * 100
	return readers
end
print("scaled", scaled(7)[1](), scaled(7)[3]())

local function sliders(v)
	local cur = v
	local api = {}
	for i = 1, 2 do
		api["set" .. i] = function(z)
			cur = z + i
		end
	end
	cur = cur * 3
	local before = cur
	api.set2(40)
	return before, cur
end
print("sliders", sliders(1))

local function swapAfter(p, q, n)
	local closures = {}
	local left, right = p, q
	for i = 1, n do
		closures[i] = function()
			return left .. right
		end
	end
	left, right = right, left
	return closures
end
print("swap", swapAfter("L", "R", 1)[1]())

local function tipJar(n)
	local jar = 0
	for i = 1, n do
		local tip = function(x)
			jar = jar + x
		end
		tip(i * 2)
	end
	return jar
end
local function streaks(list)
	local gold, hits = 0, 0
	for _, v in ipairs(list) do
		local function award()
			gold += v
			hits += 1
		end
		if v % 2 == 0 then
			award()
		end
	end
	return gold, hits
end
local function breadcrumbs(n)
	local path = ""
	local step = 0
	repeat
		step += 1
		walker = function()
			path = path .. step
		end
		walker()
	until step >= n
	return path
end
print(tipJar(4), streaks({ 1, 2, 3, 4 }), breadcrumbs(4))

local steps = 0
local function stepper(list)
	return function(_, i)
		steps += 1
		i = (i or 0) + 1
		if list[i] ~= nil then
			return i, list[i]
		end
		return nil
	end
end
local function walk(list)
	local peek = function()
		return steps
	end
	steps = 100
	local seen = {}
	for _, v in stepper(list) do
		seen[#seen + 1] = v .. "@" .. peek()
	end
	return table.concat(seen, ","), steps
end
print(walk({ "a", "b", "c" }))
print(walk({}))
