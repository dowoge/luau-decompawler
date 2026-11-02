-- The same eight fields as a template or, with one bracketed key, as stores: a
-- runtime nil keeps its template node, but as a store it leaves a node to take.
local function dump(t)
	local parts = {}
	for k, v in pairs(t) do
		table.insert(parts, tostring(k) .. "=" .. tostring(v))
	end
	return table.concat(parts, ",")
end

local function asTemplate(v1, v2, v3, v4, v5, v6, v7, v8)
	return { Text = v1, Font = v2, TextSize = v3, TextColor3 = v4, BackgroundTransparency = v5, BorderSizePixel = v6, LayoutOrder = v7, AutomaticSize = v8 }
end

local function asStores(v1, v2, v3, v4, v5, v6, v7, v8)
	return { ["Text"] = v1, Font = v2, TextSize = v3, TextColor3 = v4, BackgroundTransparency = v5, BorderSizePixel = v6, LayoutOrder = v7, AutomaticSize = v8 }
end

for hole = 0, 8 do
	local v = {}
	for i = 1, 8 do
		v[i] = if i == hole then nil else i
	end
	print(hole, dump(asTemplate(table.unpack(v, 1, 8))))
	print(hole, dump(asStores(table.unpack(v, 1, 8))))
end

local function lastBracketed(value)
	return {
		Text = 1,
		Font = 2,
		TextSize = 3,
		TextColor3 = 4,
		BackgroundTransparency = 5,
		BorderSizePixel = 6,
		LayoutOrder = 7,
		["AutomaticSize"] = value,
	}
end
local last = lastBracketed(nil)
print(dump(last), next(last, "AutomaticSize"))
last.Parent = 9
print(dump(last))
print(dump(lastBracketed(false)))
print(dump(lastBracketed(8)))
