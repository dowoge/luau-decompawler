-- A constructor whose branch item writes a captured key still stores its open
-- call tail with one SETLIST: the array part is sized exactly, which # shows.
local function spread()
	return "b", "c", "d", "e"
end

local function assemble(tag)
	local function retag()
		tag = "late"
		return true
	end
	return { [tag] = 1, kind = "row", spare = nil, if retag() then "a" else "z", spread() }
end

local row = assemble("early")
row[7] = "g"
print(#row, row.early, row.late, row.kind)
for key, value in next, row do
	print(key, value)
end
