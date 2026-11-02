-- A slot-filling helper inlined with constant arguments that drop some of its stores:
-- the table keeps the array size its stores predict, so the length operator sees it.
local function bag(weapon, armor, charm)
	local held = {}
	held[1] = weapon
	if armor ~= nil then
		held[2] = armor
	end
	if charm ~= nil then
		held[3] = charm
	end
	held[4] = "pouch"
	return held
end

local function pairUp(left, right)
	local p = {}
	if left then
		p[1] = left
	end
	p[2] = right
	return p
end

local kits = {
	bag("axe", nil, "opal"),
	bag("bow", "mail", "jade"),
	bag(nil, nil, nil),
	bag("staff", "robe", nil),
}
for i, kit in ipairs(kits) do
	print(i, #kit, kit[1], kit[2], kit[3], kit[4])
end
local hands = pairUp(nil, "shield")
local both = pairUp("sword", "shield")
print(#hands, hands[2], #both, both[1])
