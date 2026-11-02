-- `local t = {}` is sized from the field stores written after it; lookups
-- before the first store see an empty table, and stores through a helper do not count.
local function dump(t)
	local parts = {}
	for k, v in pairs(t) do
		table.insert(parts, tostring(k) .. "=" .. tostring(v))
	end
	return table.concat(parts, ",")
end

local stats = {}
stats.Kills = 1
stats.Deaths = 2
stats.Assists = 3
stats.Score = 4
stats.Streak = 5
print(dump(stats))

local function assign(t, k, v)
	t[k] = v
end
local function listAll(t)
	for k, v in pairs(t) do
		print(k, v)
	end
end
local bag = {}
print(next(bag))
print((pcall(function()
	return next(bag, "Kills")
end)))
print((pcall(next, bag, {})))
bag.Kills = 0
print(next(bag, "Kills"))
assign(bag, "Deaths", 1)
assign(bag, "Assists", 2)
assign(bag, "Score", 3)
assign(bag, "Streak", 4)
assign(bag, "Rank", 5)
assign(bag, "Title", 6)
assign(bag, "Clan", 7)
listAll(bag)
assign(bag, "Kills", nil)
listAll(bag)

local Shopkeeper = {}
Shopkeeper.Name = "Mira"
function Shopkeeper.greet(who)
	return Shopkeeper.Name .. " greets " .. who
end
Shopkeeper.Stock = { Potion = 3, Scroll = 1 }
Shopkeeper.Prices = { Potion = 10, Scroll = 45 }
Shopkeeper.Motto = string.format("%d%% off today", 20)
Shopkeeper.Open = true
Shopkeeper.Hours = { 9, 17 }
function Shopkeeper.cost(item, n)
	return Shopkeeper.Prices[item] * n
end
Shopkeeper.Town = "Harbor"
Shopkeeper.Rating = 4.5
Shopkeeper.Visits = 0
Shopkeeper.Guild = nil
Shopkeeper.Ledger = {}
Shopkeeper.Tax = 0.05
Shopkeeper.Rival = "Oren"
Shopkeeper.Pet = "cat"
Shopkeeper.Theme = "blue"
local names = {}
for name in pairs(Shopkeeper) do
	table.insert(names, name)
end
print(table.concat(names, ","))
print(Shopkeeper.greet("you"), Shopkeeper.cost("Scroll", 2), Shopkeeper.Motto)
