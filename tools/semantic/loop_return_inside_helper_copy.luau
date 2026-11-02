-- Helpers that leave from inside a loop, copied into callers: the early exit leaves only
-- the copy, a hand-written look-alike keeps looping, and values returned mid-loop land.
local strict = true
local function scanForKey(bag)
	local i = 0
	while i < #bag do
		i += 1
		if strict and bag[i] == "key" then
			return
		end
	end
	print("no key among", #bag)
end

local function openChest(bag)
	scanForKey(bag)
	print("chest checked", #bag)
end

local function openChestByHand(bag)
	local i = 0
	while i < #bag do
		i += 1
		if strict and bag[i] == "key" then
			print("chest checked", i)
		end
	end
	print("no key among", #bag)
	print("chest checked", #bag)
end

local loot = { "gem", "key", "ore", "key" }
openChest(loot)
openChestByHand(loot)
strict = false
openChest(loot)

local function hop(chain, steps)
	local cur = chain
	local taken = 0
	repeat
		taken += 1
		cur = cur.next
		if cur == nil then
			return false
		end
	until taken >= steps
	return cur
end

local function product(values, cap)
	local running = 1
	for i = 1, #values do
		running *= values[i]
		if running >= cap then
			return cap
		end
	end
	return running
end

local chain = { id = 1, next = { id = 2, next = { id = 3 } } }
local function inspect(steps, groups)
	local far = hop(chain, steps)
	local beyond = hop(chain, steps + 5)
	local small = product(groups[1], 1000)
	local large = product(groups[2], 1000)
	return far and far.id, beyond, small, large
end
print(inspect(2, { { 2, 3, 4 }, { 10, 20, 30 } }))
print(inspect(1, { {}, { 999, 2 } }))
