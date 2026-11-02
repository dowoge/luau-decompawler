-- Helpers whose result is stored back into the very local passed in: the path that
-- returns the argument unchanged, a twice-read argument and a meddling callback all hold.
local function widen(buffer, need, info)
	if info.cap >= need then
		return buffer
	end
	info.cap = need + 4
	local bigger = table.create(info.cap, "")
	table.move(buffer, 1, #buffer, 1, bigger)
	info.moves += 1
	return bigger
end

local function heavier(a, b)
	if b.kg <= a.kg then
		return a
	end
	return b
end

local function shift(p, d)
	return { x = p.x + d.x, y = p.y * d.y }
end

local function compose(words)
	local info = { cap = 2, moves = 0 }
	local buffer = {}
	local i = 0
	while i < #words do
		i += 1
		buffer = widen(buffer, i + 1, info)
		buffer[i] = words[i]
	end
	buffer = widen(buffer, #words + 6, info)
	return #buffer, info.cap, info.moves, buffer[2]
end

local function heaviest(crates)
	local best = { id = "empty", kg = 0 }
	for _, crate in ipairs(crates) do
		best = heavier(best, crate)
	end
	return best.id
end

local function drift(p, steps)
	for i = steps, 1, -1 do
		p = shift(p, { x = i, y = 2 })
	end
	return p.x, p.y
end

local function rehome(buffer)
	local info = { cap = 0, moves = 0 }
	local fresh = widen(buffer, 3, info)
	local same = buffer
	same = widen(fresh, 1, info)
	return same == fresh, #same, info.moves
end

local function sanitize(text, rule)
	if rule(text) then
		return text
	end
	return "[" .. text .. "]"
end

local function rewrite()
	local text = "raw"
	local calls = 0
	local function rule(seen)
		calls += 1
		text = "rule saw " .. seen
		return false
	end
	text = sanitize(text, rule)
	return text, calls
end

print(compose({ "a", "b", "c", "d", "e" }))
print(compose({}))
print(heaviest({ { id = "x", kg = 3 }, { id = "y", kg = 9 }, { id = "z", kg = 9 } }), heaviest({}))
print(drift({ x = 0, y = 1 }, 3))
print(rehome({ "k" }))
print(rewrite())
