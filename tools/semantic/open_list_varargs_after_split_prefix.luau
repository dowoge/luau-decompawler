-- Field entries, then a branch whose arms log through an inlined helper, then
-- `...`: the varargs still land in one bulk store, nils and all.
local log = {}
local function note(v)
	log[#log + 1] = tostring(v)
	return v
end

local function pack(flag, ...)
	return { mode = flag, label = note("L"), label = nil, if flag then note(1) else note(0), ... }
end

local function inspectRow(t)
	t[9] = "x"
	print(#t, table.concat(log, ","))
	for key, value in next, t do
		print(type(key), key, value)
	end
	table.clear(log)
end

inspectRow(pack(true, 2, 3, 4, 5, 6))
inspectRow(pack(false, 7, nil, 9))
