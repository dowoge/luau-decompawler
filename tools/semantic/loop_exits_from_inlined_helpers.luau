-- Inventory helpers return from inside loops and get inlined at -O2, jumping past
-- the rest of the helper; a break reaches the helper's tail instead.
local function slotOf(bag, name)
	for i = 1, #bag do
		if bag[i].name == name then
			return i
		end
	end
	return nil
end
local function lastWithRoom(bag, cap)
	for i = #bag, 1, -1 do
		if bag[i].count < cap then
			return i
		end
	end
	return 0
end
local function firstHeavy(bag, weight)
	for i = 1, #bag do
		if bag[i].count * 2 > weight then
			return bag[i].name
		end
	end
end
local function slotOrWarn(bag, name)
	for i = 1, #bag do
		if bag[i].name == name then
			return i
		end
	end
	print("missing", name)
	return -#bag
end
local function holds(bag, name)
	for i = 1, #bag do
		if bag[i].name == name then
			return true
		end
	end
	return false
end
local function grade(bag, count)
	for i = 1, #bag do
		if bag[i].count == count then
			return "exact"
		elseif bag[i].count > count then
			return "more"
		end
	end
	return "less"
end
local function take(bag, name)
	for i = 1, #bag do
		if bag[i].name == name then
			return i, bag[i].count - 1
		end
	end
	return nil, 0
end
local function untilEmpty(bag, name)
	for i = 1, #bag do
		if bag[i].count == 0 then
			break
		end
		if bag[i].name == name then
			return i
		end
	end
	return -1
end
local function untilEmptyIter(bag, name)
	for i, item in bag do
		if item.count == 0 then
			break
		end
		if item.name == name then
			return i
		end
	end
	return -1
end

local function item(name, count)
	return { name = name, count = count }
end
local pack = { item("rope", 2), item("lamp", 0), item("gem", 5), item("rope", 9) }

local function inspect(bag)
	local s = slotOf(bag, "gem")
	local r = lastWithRoom(bag, 3)
	local h = firstHeavy(bag, 7)
	print(s, r, h, firstHeavy(bag, 100))
end
local function audit(bag, a, b)
	local x = slotOrWarn(bag, a)
	local y = slotOrWarn(bag, b)
	if holds(bag, a) then
		print("holds", a)
	end
	print(x, y, grade(bag, 2), grade(bag, 50))
end
local function sweep(bags, name)
	for n, bag in bags do
		local at = slotOf(bag, name)
		if at then
			print("found", n, at)
			break
		end
	end
	for n = 1, #bags do
		local i, left = take(bags[n], name)
		print(n, i, left)
	end
	local n = 0
	while n < #bags do
		n += 1
		print(n, untilEmpty(bags[n], name), untilEmptyIter(bags[n], name))
	end
end

inspect(pack)
inspect({})
audit(pack, "rope", "axe")
audit({ item("axe", 1) }, "axe", "gem")
sweep({ { item("lamp", 1) }, pack, { item("gem", 0) } }, "gem")

local function gather(t)
	local picked = {}
	local stopAt = nil
	local sum = 0
	for k, v in pairs(t) do
		if v.stop then
			stopAt = k
			break
		end
		picked[#picked + 1] = v
		sum += v.n
	end
	return #picked, stopAt, sum
end
print(gather({}))
print(gather({ a = { n = 4 } }))
print(gather({ a = { n = 4, stop = true } }))
