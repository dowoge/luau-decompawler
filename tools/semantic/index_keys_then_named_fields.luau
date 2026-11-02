-- `{[1] = nil, [2] = .., [3] = ..}` gets an array part and no hash part; the
-- field stores after it grow the hash later, which pairs order shows.
local function dump(t)
	local parts = {}
	for k, v in pairs(t) do
		table.insert(parts, tostring(k) .. "=" .. tostring(v))
	end
	return table.concat(parts, ",")
end

local t = { [1] = nil, [2] = "b", [3] = "c" }
print(dump(t), #t)
t.Title = 1
t.Color = 2
t.Parent = 3
t[25] = 25
t[3] = nil
t[26] = 26
print(dump(t))
