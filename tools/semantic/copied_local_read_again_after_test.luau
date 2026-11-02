-- a local copied into another one and read again only by the copy's
-- expression: it stays a local
local Palette = { OWNED = { { 10, 20, 30 }, { 40, 50, 60 } } }
local calls = 0
local function style()
	calls += 1
	return select(1, 1)
end
local function pick(arg, value)
	local s = arg or "main"
	if value == nil and style() == 1 then
		local slot = s == "main" and 1 or s == "alt" and 2 or nil
		local own = slot and Palette.OWNED[slot]
		if own ~= nil then
			value = own[1] + own[3]
		end
	end
	return value
end
local function first(list, key)
	local i = key and #list or 1
	local v = i and list[i]
	return v
end
local args = { { "main" }, { "alt" }, { "x" }, { nil, 5 } }
for _, a in args do
	print(pick(a[1], a[2]), calls)
end
for _, k in { true, false } do
	print(first({ "a", "b", "c" }, k))
end
