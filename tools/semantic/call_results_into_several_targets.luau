-- One call feeding several targets at once: locals assigned crosswise, a target named
-- twice, and upvalues filled from inside closures all keep their own results.
local function divmod(n, d)
	return n // d, n % d
end

local function threeParts(word)
	return word:sub(1, 1), word:sub(2, 2), word:sub(3)
end

local out = {}
local function emit(...)
	local parts = table.pack(...)
	for i = 1, parts.n do
		parts[i] = tostring(parts[i])
	end
	out[#out + 1] = table.concat(parts, " ", 1, parts.n)
end

for _, pair in ipairs({ { 29, 4 }, { 6, 7 } }) do
	local first, second = pair[1], pair[2]
	second, first = divmod(first, second)
	local peek = function()
		return first
	end
	if second > 0 then
		first, second = second, first
	end
	emit("cross", first, second, peek())
end

local function scramble(word, keep)
	local x, y, z = threeParts(word)
	if keep then
		y = y:upper()
	else
		z, x = x, z
		while #z < 4 do
			z = z .. "."
		end
	end
	z, z = divmod(#word * 7, 4)
	return x .. y, z
end
emit("scramble", scramble("lumen", true))
emit("scramble", scramble("ox", false))

local function account(address)
	local user, host = string.match(address, "^([^@]+)@(.+)$")
	if host == "localhost" then
		host = user
	end
	return user, host
end
emit("account", account("kai@example.org"))
emit("account", account("root@localhost"))

local whole, rest
local runs = 0
local function measure(n)
	runs += 1
	whole, rest = divmod(n, 6)
end
measure(40)
measure(45)
emit("upvalues", whole, rest, runs)

local a, b, c = "red", "green", "blue"
local function rotate()
	a, b, c = b, c, a
end
rotate()
rotate()
emit("rotate", a, b, c)

local text = "pins 4 and 15 then 230"
local from, to = 0, 0
local function nextNumber()
	from, to = string.find(text, "%d+", to + 1)
	return from
end
local found = {}
while nextNumber() do
	found[#found + 1] = text:sub(from, to)
end
emit("numbers", #found, table.concat(found, ","))

for _, line in ipairs(out) do
	print(line)
end
