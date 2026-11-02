-- The open call ending a constructor stores every result in place, nils
-- included, over a keyed entry stored earlier in the same constructor.
local seen = {}
local function note(label, value)
	seen[#seen + 1] = label
	return value
end

local function results()
	note("results")
	return nil, "two", nil, "four"
end

local row = { note("first", "one"), [2] = note("keyed", "old"), results() }
print(row[1], row[2], row[3], row[4], row[5], table.concat(seen, ","))
