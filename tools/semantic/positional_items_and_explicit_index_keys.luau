-- Explicit `[n]` keys next to list items: pending list items are stored before
-- each keyed entry, so the keyed one wins, and `[n]` keys keep their slots.
local function drained()
	local q = { "list", [1] = "keyed" }
	local before = q[1]
	q[1] = nil
	return before, q[1]
end
print(drained())

local Quests = {}
Quests.Active = {}
Quests.MaxStage = 4
Quests.StageNames = { "Gather", "Hunt", "Forge", "Return" }
Quests.Rewards = {
	[1] = { { Item = "Coin", Count = 5 } },
	[2] = { { Item = "Coin", Count = 15 } },
	[3] = { { Item = "Gem", Count = 1 } },
	[4] = { { Item = "Gem", Count = 3 } },
	Bonus = { { Item = "Crown", Count = 1 } },
}
Quests.Weights = { 3, 6, 9 }
print("defining")
function Quests.Stage(n)
	return Quests.StageNames[n]
end
function Quests.Reward(n)
	return Quests.Rewards[n][1].Count
end
function Quests.Weight(n)
	return Quests.Weights[n] or 0
end
local seen = {}
for key, value in pairs(Quests.Rewards) do
	table.insert(seen, tostring(key) .. ":" .. value[1].Item)
end
table.sort(seen)
print(table.concat(seen, ","), #Quests.Rewards, Quests.Stage(2), Quests.Reward(4), Quests.Weight(5))

local function compose(seed, flip, ...)
	local sign = if flip then 1 else -1
	local bumped = seed * 3
	local t = {
		dir = sign,
		["note"] = nil,
		[2] = if flip then seed else bumped,
		["note"] = nil,
		"head",
		sign,
		if flip then "hi" else "lo",
		...,
	}
	return t
end
for _, flip in { true, false } do
	for _, extra in { 0, 7, 9 } do
		local t = compose(2, flip, "v", nil)
		if extra > 0 then
			t[extra] = extra
		end
		print(#t, t[1], t[2], t[3], t[4], t[5])
		for key, value in next, t do
			print(type(key), key, value)
		end
	end
end
