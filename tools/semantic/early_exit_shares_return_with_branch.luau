-- An early loop exit and a sibling branch that returns end in one shared `return`,
-- while running out of items goes on elsewhere; every effect and result must match.
local notes = {}
local function note(v)
	notes[#notes + 1] = tostring(v)
end

local function openChest(kind, loot, luck)
	if kind == "chest" then
		for _, roll in ipairs(loot) do
			if roll > luck then
				break
			end
			note("gem" .. roll)
		end
	elseif kind == "mimic" then
		note("bite")
		return
	else
		note("dust")
	end
end

local function countCoins(kind, loot, luck)
	if kind ~= "chest" then
		if kind == "mimic" then
			note("bite")
			return
		end
		note("dust")
		return
	end
	for i = 1, #loot do
		if loot[i] > luck then
			break
		end
		note(loot[i] * 10)
	end
end

local function drainPotions(kind, loot, luck)
	if kind == "chest" then
		local i = 1
		while i <= #loot do
			if loot[i] > luck then
				break
			end
			note("sip" .. loot[i])
			i += 1
		end
	elseif kind == "mimic" then
		return note("bite")
	end
end

local function rollDice(kind, loot, luck)
	if kind == "mimic" then
		note("bite")
		return
	elseif kind == "chest" then
		local i = 0
		repeat
			i += 1
			if i > #loot or loot[i] > luck then
				break
			end
			note(-loot[i])
		until i >= #loot
	else
		note("dust")
	end
end

local function settle(v)
	note("settle")
	return v, #notes, "end"
end

local function bestPull(kind, loot, luck)
	local pulled = "none"
	if kind == "chest" then
		for _, roll in ipairs(loot) do
			pulled = roll
			if roll > luck then
				return settle(pulled)
			end
			note(roll)
		end
		note("half")
		for i = #loot, 1, -1 do
			pulled = loot[i] + 1
			if pulled > luck then
				return settle(pulled)
			end
			note(pulled)
		end
		note("spent")
		return "clean"
	end
	return settle(pulled)
end

local function watchers(kind, loot, luck)
	local list = {}
	if kind == "chest" then
		for i, roll in ipairs(loot) do
			list[#list + 1] = function()
				return i, roll
			end
			for bonus = 1, 2 do
				if roll + bonus > luck then
					note("cut")
					return list
				end
				note(roll + bonus)
			end
		end
		note("all")
		return list
	end
	note("skipped")
	return list
end

for _, loot in ipairs({ {}, { 2 }, { 2, 4, 6 } }) do
	for _, kind in ipairs({ "chest", "mimic", "urn" }) do
		for _, luck in ipairs({ 1, 4, 9 }) do
			for _, run in ipairs({ openChest, countCoins, drainPotions, rollDice, bestPull }) do
				notes = {}
				local a, b, c = run(kind, loot, luck)
				print(#loot, kind, luck, table.concat(notes, ","), a, b, c)
			end
			notes = {}
			for _, get in ipairs(watchers(kind, loot, luck)) do
				local i, roll = get()
				note(i .. "=" .. roll)
			end
			print(#loot, kind, luck, table.concat(notes, ","))
		end
	end
end
