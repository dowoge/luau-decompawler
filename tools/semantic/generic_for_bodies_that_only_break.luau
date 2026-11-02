-- A generic `for` whose body only breaks still makes its first iterator call, and a swap in
-- the enclosing loop around such a `for` happens exactly once per outer trip.
local pulls = 0
local function deck(size)
	local drawn = 0
	return function()
		pulls += 1
		drawn += 1
		if drawn <= size then
			return drawn
		end
		return nil
	end
end
local function hasAny(hand)
	for _ in hand do
		break
	end
	return "checked"
end
local function drawOnce()
	for _ in deck(5) do
		if 3 > 1 then
			break
		end
	end
	return pulls
end
print(hasAny({ "ace" }), drawOnce(), drawOnce())

local function shuffleGeneric(seed, hand)
	local top, bottom = seed, "-"
	for _ in hand do
		top, bottom = bottom, top
		for _ in hand do
			break
		end
	end
	return top, bottom
end
local function shuffleCounted(seed, times, hand)
	local top, bottom = seed, "-"
	for _ = 1, times do
		top, bottom = bottom, top
		for _, _ in ipairs(hand) do
			break
		end
	end
	return top, bottom
end
local function shufflePiles(piles)
	local top, bottom, cards = "up", "down", 0
	for _, pile in ipairs(piles) do
		top, bottom = bottom, top
		cards += #pile
		for _, _ in ipairs(pile) do
			break
		end
	end
	return top, bottom, cards
end
print(shuffleGeneric("K", {}), shuffleGeneric("K", { 1 }), shuffleGeneric("K", { 1, 2 }))
print(shuffleCounted("Q", 0, { 1 }), shuffleCounted("Q", 2, {}), shuffleCounted("Q", 3, { 7 }))
print(shufflePiles({ { 1, 2 }, {}, { 3 } }))
