-- A script-shaped chunk: state set up front, a retry loop, an event handler that
-- writes a module cell, and a refresh that reads it from inside its own loop.
local labels = {}
local Player = { Backpack = nil, Coins = 40 }
local remote = nil
local maxTries = 3
local Bought = { queue = {} }
Bought.OnClientEvent = {
	Connect = function(_, fn)
		table.insert(Bought.queue, fn)
	end,
}
local Catalog = {
	Size = function()
		return 4
	end,
	IsOwned = function(n)
		return n <= 2
	end,
}
local Hud = {
	Show = function(tier, price)
		labels[#labels + 1] = tostring(tier) .. "@" .. tostring(price)
	end,
}
local wantedTier, chosenItem, priceLookup = 3, nil, nil
local shownCoins, lastCoins, shownPrice = Player.Coins, 0, 25
local activeTier = nil

local tries = 0
while not remote and tries < maxTries do
	tries += 1
	if tries == 2 then
		labels[#labels + 1] = "retry"
	end
end

if Bought then
	Bought.OnClientEvent:Connect(function(ok: boolean, note: string, tier: number?)
		if ok and tier then
			for t = tier + 1, Catalog.Size() do
				if not Catalog.IsOwned(t) then
					labels[#labels + 1] = "next" .. t
					break
				end
			end
			activeTier = wantedTier
		end
	end)
end

local function priceOf(tier: number?): number?
	if not tier or not priceLookup then
		return nil
	end
	return priceLookup(tier)
end

local function refresh(profile: any)
	if profile and profile.Owned then
		for t = 2, Catalog.Size() do
			if profile.Owned[t] then
				continue
			end
			if shownCoins ~= lastCoins then
				Hud.Show(activeTier, priceOf(t) or shownPrice)
			end
		end
		local function redraw()
			return chosenItem
		end
		redraw()
	end
end

refresh({ Owned = { [3] = true } })
for _, fn in Bought.queue do
	fn(true, "", 1)
end
priceLookup = function(tier)
	return tier * 10
end
refresh({ Owned = {} })
print(tries, table.concat(labels, ","))
