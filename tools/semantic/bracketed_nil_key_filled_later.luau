-- A bracketed key stored nil first (its node free for later keys) and filled
-- afterwards, or filled first and cleared afterwards, keeps the stores' layout.
local function dump(t)
	local parts = {}
	for k, v in pairs(t) do
		table.insert(parts, tostring(k) .. "=" .. tostring(v))
	end
	return table.concat(parts, ",")
end

local filled = { ["Owner"] = nil, Pet = 2, Mount = 3, Weapon = 4, Shield = 5, Helmet = 6, Boots = 7, Ring = 8 }
filled.Owner = 1
print(dump(filled))

local cleared = { ["Owner"] = 0, Pet = 2, Mount = 3, Weapon = 4, Shield = 5, Helmet = 6, Boots = 7, Ring = 8 }
cleared.Owner = nil
print(dump(cleared))

local computed = {
	["Owner"] = nil,
	Pet = tonumber("2"),
	Mount = tonumber("3"),
	Weapon = tonumber("4"),
	Shield = tonumber("5"),
	Helmet = tonumber("6"),
	Boots = tonumber("7"),
	Ring = tonumber("8"),
}
computed.Owner = 1
print(dump(computed))
