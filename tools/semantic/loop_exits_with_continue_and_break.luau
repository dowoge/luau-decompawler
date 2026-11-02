-- Loops left by `continue`, `break`, an `until` test, a `return` out of nested repeats,
-- or an inner loop back to an endless outer one: every exit must land where the source sent it.
local function packBackpack(items, capacity)
	local packed, weight = {}, 0
	local i = 0
	repeat
		i += 1
		local item = items[i]
		if item == nil or item.junk then
			continue
		end
		if weight + item.mass > capacity then
			break
		end
		weight += item.mass
		packed[#packed + 1] = item.name
	until i >= #items
	return table.concat(packed, ","), weight, i
end
print(packBackpack({
	{ name = "rope", mass = 2 },
	{ name = "rock", junk = true, mass = 9 },
	{ name = "lamp", mass = 3 },
	{ name = "anvil", mass = 50 },
	{ name = "gem", mass = 1 },
}, 10))

local function awaitCharacter(player)
	local polls = 0
	repeat
		polls += 1
		if not player.Character then
			player.Character = polls >= 3 and { Health = 100 } or nil
			continue
		end
		if player.Character.Health <= 0 then
			break
		end
		player.Character.Health -= 40
	until polls > 8 or player.Left
	return polls, player.Character and player.Character.Health
end
print(awaitCharacter({}))
print(awaitCharacter({ Character = { Health = 0 } }))
print(awaitCharacter({ Left = true }))

local function spawnWaves(limit)
	local spawned = {}
	local wave = 0
	while true do
		wave += 1
		if wave % 4 == 0 then
			if #spawned >= limit then
				break
			end
			spawned[#spawned + 1] = "boss" .. wave
		end
		if wave >= 14 then
			break
		end
	end
	return table.concat(spawned, ","), wave
end
print(spawnWaves(2), spawnWaves(9))

local function chargeShots(cap)
	local shots = {}
	local t = 0
	repeat
		t += 1
		if t % 3 ~= 0 then
			if t > cap then
				break
			end
			shots[#shots + 1] = t
		end
	until t >= 9
	return table.concat(shots, " "), t
end
print(chargeShots(5))
print(chargeShots(99))

local function seatFor(ticket, maxRow)
	local rowNo, seatNo = 0, 0
	repeat
		rowNo += 1
		if maxRow and rowNo > maxRow then
			break
		end
		repeat
			seatNo += 1
			if seatNo == ticket then
				return ("R%dS%d"):format(rowNo, seatNo)
			end
		until seatNo >= rowNo * 2
	until rowNo >= 3
	return "standing"
end
print(seatFor(1), seatFor(5), seatFor(40), seatFor(3, 1), seatFor(3, 2))

local function reload(mode)
	local fired = {}
	local rounds, ticks = 0, 0
	repeat
		ticks += 1
		if mode == "burst" then
			repeat
				rounds += 1
			until rounds >= 4 or mode ~= "burst"
			repeat
				if rounds % 2 == 1 then
					fired[#fired + 1] = rounds
				end
				rounds += 1
			until rounds > 9 or rounds == 3
			if ticks > 10 then
				break
			end
		end
	until ticks % 3 == 0
	return table.concat(fired, ","), rounds, ticks
end
print(reload("burst"))
print(reload("single"))

local function decay(hp, rate, floor)
	local stacks = 1
	while true do
		if hp % 2 == 0 then
			hp /= 2
		else
			hp = (hp - 1) / 2
			stacks = stacks * rate
			if hp < floor then
				return stacks, hp
			end
		end
		rate += 1
	end
end
print(decay(13, 2, 1), decay(40, 3, 2))

local heartbeat = 0
local frames = {}
local function pulse()
	heartbeat += 1
	if heartbeat > 18 then
		error("server closed", 0)
	end
	return heartbeat
end
local function serverLoop()
	while true do
		if pulse() % 4 == 1 then
			while true do
				if pulse() % 3 == 0 then
					frames[#frames + 1] = "save" .. heartbeat
					break
				end
				for lane = 1, 2 do
					frames[#frames + 1] = "lane" .. lane
				end
				if pulse() % 7 == 0 then
					break
				end
			end
		end
	end
end
print(pcall(serverLoop))
print(table.concat(frames, ","))

local hits = {}
for wave = 1, 3 do
	repeat
		hits[#hits + 1] = "w" .. wave
	until true
	break
end
for _, door in ipairs({ "red", "blue" }) do
	repeat
		if door == "red" then
			break
		end
		hits[#hits + 1] = "unreached"
	until true
	break
end
for _, key in ipairs({ "gold", "iron" }) do
	repeat
		hits[#hits + 1] = key
	until { key }
	break
end
local floor = 0
while floor < 3 do
	floor += 1
	for room = 1, 3 do
		if room == 2 then
			break
		end
		hits[#hits + 1] = floor .. "-" .. room
	end
	break
end
print(table.concat(hits, ","))
