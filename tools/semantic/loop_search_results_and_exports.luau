-- Values found in loops leave through `break` or exhaustion: a skipped search keeps the old
-- value, an exhausted one stores nil, and closures made after the loop share the found cell.
local Quests = {}
function Quests.active(log, wanted, current)
	if wanted == nil then
		return current
	end
	local hit = nil
	if log then
		for _, id in log do
			if id == wanted then
				hit = id
				break
			end
		end
	end
	current = hit
	return current
end
print(Quests.active({ 3, 6 }, nil, "keep"), Quests.active(nil, 6, "keep"), Quests.active({ 3, 6 }, 6, "keep"), Quests.active({ 3 }, 9, "keep"))

local function lookup(pool, wanted)
	for _, entry in pool do
		if entry == wanted then
			return entry
		end
	end
	return nil, false
end
local function refresh(pool, wanted, enabled, cached, ok)
	if enabled then
		cached, ok = lookup(pool, wanted)
	end
	return cached, ok
end
print(refresh({}, 5, true, "old", "yes"))
print(refresh({ 4, 5 }, 5, true, "old", "yes"))
print(refresh({ 4 }, 5, true, "old", "yes"))
print(refresh({ 5 }, 5, false, "old", "yes"))

local function scanShelves(shelves, wanted)
	local found = {}
	local lastShelf
	for s, shelf in ipairs(shelves) do
		local item, slot = nil, 0
		for i, v in ipairs(shelf) do
			if v == wanted then
				item, slot = v, i
				break
			end
		end
		if item then
			found[#found + 1] = s .. ":" .. slot
			lastShelf = s
		end
	end
	return table.concat(found, " "), lastShelf
end
print(scanShelves({ { "axe" }, { "rope", "axe", "axe" }, { "map" } }, "axe"))
print(scanShelves({}, "axe"))

local function prune(owned, roster)
	for id in pairs(owned) do
		local member = nil
		for _, m in ipairs(roster) do
			if m.Id == id then
				member = m
				break
			end
		end
		if member == nil then
			owned[id] = nil
		else
			owned[id] = member.Rank
		end
	end
	local left = {}
	for id, rank in pairs(owned) do
		left[#left + 1] = id .. "=" .. rank
	end
	table.sort(left)
	return table.concat(left, ",")
end
print(prune({ u1 = true, u2 = true, u3 = true }, { { Id = "u3", Rank = 2 }, { Id = "u1", Rank = 7 } }))

local function follow(npcs, name)
	local target = nil
	for _, npc in npcs do
		if npc.Name == name then
			target = npc
			break
		end
	end
	local function label()
		return target and target.Name or "-"
	end
	local function retarget(npc)
		target = npc
	end
	local function hud()
		return function()
			return label() .. (target and "*" or "?")
		end
	end
	return label, hud(), retarget
end
for _, name in { "Guard", "Smith", "Ghost" } do
	local label, hud, retarget = follow({ { Name = "Guard" }, { Name = "Smith" } }, name)
	print(label(), hud())
	retarget({ Name = "Mayor" })
	print(label(), hud())
	retarget(nil)
	print(label(), hud())
end
