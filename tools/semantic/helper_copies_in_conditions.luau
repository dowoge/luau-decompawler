-- Helper copies used as conditions, negated, inside `and`, and with function-literal
-- arguments spanning several lines: every condition and every callback behaves the same.
local function isVowel(ch)
	return ch:find("[aeiou]") ~= nil
end

local function consonantRuns(word)
	local runs, inRun = 0, false
	for i = 1, #word do
		local ch = word:sub(i, i)
		if not isVowel(ch) and ch ~= "-" then
			if not inRun then
				runs += 1
			end
			inRun = true
		else
			inRun = false
		end
	end
	return runs, isVowel(word:sub(1, 1)), not isVowel(word:sub(-1))
end
print(consonantRuns("strength-test"))
print(consonantRuns("aerie"))

local registry = {}
local function keep(owner, handle)
	local list = registry[owner]
	if list == nil then
		list = {}
		registry[owner] = list
	end
	list[#list + 1] = handle
end
local aliases = setmetatable({}, {
	__index = function()
		return keep
	end,
})
local Door = {}
Door.__index = Door
function Door:OnOpen(fn)
	fn("creak")
	return { live = true, fn = fn }
end
local door = setmetatable({}, Door)
local function hookUp(guest)
	keep(guest.Seat, door:OnOpen(function(sound)
		print("heard", sound, guest.Seat)
	end))
	keep(guest.Seat, door:OnOpen(function()
		print("left", guest.Seat)
	end))
	keep(guest.Seat, {
		live = guest.Seat % 2 == 0,
		kind = "note",
		data = { 1, 1, 2, 3, 5, 8, 13, 21 },
		nested = { deep = { deeper = true } },
	})
	registry.handmade = registry.handmade or {}
	print("halfway", guest.Seat)
	table.insert(registry.handmade, guest.Seat)
end
hookUp({ Seat = 4 })
hookUp({ Seat = 7 })
aliases.store(7, "loose")
for _, key in ipairs({ 4, 7, "handmade" }) do
	local list = registry[key]
	print(key, #list, type(list[1]), type(list[#list]))
end

local function place(name, up)
	local n = { name = name, up = up, tags = {} }
	function n:Within(region)
		local cur = self.up
		while cur ~= nil do
			if cur == region then
				return true
			end
			cur = cur.up
		end
		return false
	end
	function n:Tag(k, v)
		self.tags[k] = v
	end
	return n
end

local land = place("Land")
local sea = place("Sea")
sea:Tag("depth", 9)
local regions = { land, sea }

local function charted(x)
	for i = 1, #regions do
		if x:Within(regions[i]) then
			return true
		end
	end
	return false
end

local journal = {}
local function mark(x, n)
	x:Tag("order", n)
	journal[#journal + 1] = x.name .. "=" .. n
end

local function depthScore(x, k)
	return #x.name * k + regions[2].tags.depth
end

local function survey(strict, x, others)
	if strict and charted(x) then
		return "charted " .. x.name
	end
	for i = 1, #others do
		mark(others[i], i)
	end
	mark(x, 0)
	if charted(others[1]) then
		mark(others[1], -1)
	end
	return depthScore(x, 1) * depthScore(x, 2)
end

local boat = place("Boat", sea)
local rock = place("Rock")
local hut = place("Hut", land)
print(survey(true, boat, { rock }), survey(true, rock, { boat, hut }), survey(false, boat, { hut }))
print(table.concat(journal, " "), boat.tags.order, hut.tags.order)
