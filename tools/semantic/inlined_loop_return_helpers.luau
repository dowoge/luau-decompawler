-- A helper's `return` from inside a loop, inlined where the caller carries on, becomes a
-- flag and a `break`; rebuilt it must skip exactly what the return skipped, values included.
local eggs = {}
local hatched = false
local journal = {}
local function hatchAll()
	if hatched then
		return
	end
	for _, egg in pairs(eggs) do
		if egg.warmth < 3 then
			return
		end
	end
	hatched = true
	table.insert(journal, "hatched")
end
local function onTick(n)
	hatchAll()
	table.insert(journal, "tick" .. n)
end
local function allWarm()
	local warm = true
	for _, egg in pairs(eggs) do
		if egg.warmth < 3 then
			warm = false
			break
		end
	end
	if warm then
		table.insert(journal, "warm")
	end
	return warm
end
eggs = { { warmth = 5 }, { warmth = 1 } }
onTick(1)
print(allWarm())
eggs[2].warmth = 4
onTick(2)
onTick(3)
print(allWarm(), table.concat(journal, " "))

local function feed(appetite, treats)
	local bites = 0
	while bites < 5 do
		bites += 1
		if bites > appetite then
			return
		end
		if bites == 3 then
			break
		end
	end
	if treats > 1 then
		print("purr", treats)
	else
		print("nap")
	end
end
local function tip(appetite, treats)
	local bites = 0
	while bites < 5 do
		bites += 1
		if bites > appetite then
			return bites * 2
		end
		if bites == 3 then
			break
		end
	end
	if treats > 1 then
		return -treats
	end
	return 0
end
local function groom(appetite, strokes)
	local bites = 0
	while bites < 5 do
		bites += 1
		if bites > appetite then
			return
		end
		if bites == 3 then
			break
		end
	end
	for s = 1, strokes do
		print("stroke", s)
	end
end
local coins = 0
for pet = 1, 4 do
	if pet == 2 then
		continue
	end
	feed(pet, pet)
	coins += tip(pet, pet)
	groom(pet, pet - 1)
	print("pet", pet, coins)
end

local function findPet(pets, kind)
	for _, pet in pets do
		if pet.Kind == kind then
			return pet
		end
	end
	return nil
end
local function owns(list, kind)
	for _, k in ipairs(list) do
		if k == kind then
			return true
		end
	end
	return false
end
local function rarest(pets, tier)
	for _, pet in pets do
		if pet.Tier >= tier then
			return pet.Kind
		end
	end
	print("nothing at tier", tier)
	return "common"
end
local function equipPet(pets, kind)
	local pet = findPet(pets, kind)
	if pet then
		pet.Equipped = true
	end
	return pet
end
local function matches(list, wanted)
	local n = 0
	for _, kind in wanted do
		if owns(list, kind) then
			n += 1
		end
	end
	return n
end
local function badge(pets, tier)
	local kind = rarest(pets, tier)
	return "[" .. kind .. "]"
end
local function newest(pets, kind)
	local found = nil
	for _, pet in pets do
		if pet.Kind == kind then
			found = pet
		end
	end
	return found
end
local pets = { { Kind = "cat", Tier = 1 }, { Kind = "owl", Tier = 3 }, { Kind = "cat", Tier = 2, Shiny = true } }
print(equipPet(pets, "owl").Kind, pets[2].Equipped, equipPet(pets, "fox"))
print(matches({ "cat", "owl" }, { "owl", "fox", "cat" }), matches({}, { "cat" }))
print(badge(pets, 3), badge(pets, 5), newest(pets, "cat").Shiny)
