-- Helpers that overwrite one of their parameters, copied where the argument is a caller's
-- local: that local changes only where the source itself writes it.
local defaults = { color = "grey", size = "M" }

local function option(bag, key, otherwise)
	if key == nil then
		return otherwise
	end
	local ok, value = pcall(function()
		return bag[key]
	end)
	if ok and value ~= nil then
		otherwise = value
	end
	return otherwise
end

local function idOf(who)
	if type(who) == "table" then
		who = who.Id or who.Name
	end
	return who
end

local orders = {}
for _, req in ipairs({ { color = "color", size = "nope", by = { Id = 9 } }, { by = "anon" }, { size = "size", by = { Name = "zed" } } }) do
	orders[#orders + 1] = option(defaults, req.color, "unset") .. "/" .. option(defaults, req.size, defaults.size) .. "/" .. tostring(idOf(req.by))
end
print(table.concat(orders, " "))

local bytes = {}
local function packByte(dst, slot, value, flip)
	if value >= 64 then
		value = bit32.band(value * 3, 63)
	end
	if flip then
		value = 255 - value
	end
	dst[slot] = string.format("%02x", value)
end

local seenNames = {}
local function addName(name)
	if type(name) == "table" then
		name = name.Name
	end
	if not name or name == "" then
		return
	end
	seenNames[name] = (seenNames[name] or 0) + 1
	bytes[#bytes + 1] = name
end

local codes = { a = 10, b = 70, c = 33 }
local function packWord(word)
	local slot = 0
	for ch in string.gmatch(word, ".") do
		local num = codes[ch]
		if num then
			slot += 1
			packByte(bytes, slot, num, ch == "b")
		end
	end
	return slot
end

local function names(list)
	for i = 1, #list do
		addName(list[i])
	end
end

local function keepAfter(n)
	local num = n + 60
	packByte(bytes, 30, num, false)
	return num
end

local function heldByClosure(n)
	local num = n * 7
	local peek = function()
		return num
	end
	packByte(bytes, 31, num, true)
	return peek()
end

print(packWord("abcab"), bytes[1], bytes[2], bytes[3], bytes[5])
names({ "kit", { Name = "rex" }, "", { Name = "" }, "kit", { Name = false } })
print(#bytes, bytes[#bytes], seenNames.kit, seenNames.rex)
print(keepAfter(10), bytes[30], heldByClosure(20), bytes[31])

local cells = {}
local function store(slot, v)
	if v >= 40 then
		v -= 40
	end
	cells[slot] = v
end

local function wrapText(v)
	if v >= 40 then
		v -= 40
	end
	return "(" .. v .. ")"
end

local function settleAll(v)
	local passes = 0
	repeat
		passes += 1
		if v >= 40 then
			v -= 40
		end
		cells[passes] = v
	until passes == 3
	return v
end

local function settleOnce(v, go)
	if go then
		if v >= 40 then
			v -= 40
		end
		cells[4] = v
	end
	return v * 10
end

local function drainAll(v)
	local hops = 0
	while v >= 40 do
		hops += 1
		if v >= 40 then
			v -= 40
		end
		cells[4 + hops] = v
	end
	return hops, v
end

local function labels(v, go)
	local text = {}
	for i = 1, 2 do
		if v >= 40 then
			v -= 40
		end
		text[i] = "(" .. v .. ")"
	end
	local last = "-"
	if go then
		if v >= 40 then
			v -= 40
		end
		last = "(" .. v .. ")"
	end
	return v, table.concat(text), last
end

local function maybeStore(v, go)
	if go then
		store(9, v)
	end
end

store(10, 95)
print(settleAll(130), cells[1], cells[2], cells[3])
print(settleOnce(70, true), cells[4], cells[10])
print(drainAll(170))
print(cells[5], cells[6], cells[7], cells[8])
print(wrapText(85), labels(130, true))
print(labels(30, false))
maybeStore(77, true)
maybeStore(1, false)
print(cells[9])
