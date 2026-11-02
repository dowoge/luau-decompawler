-- A reward helper copied into table constructors with constant counts: the branch on
-- the count is decided per copy, and the text it builds must match the call's.
local function reward(count)
	local unit = "Coin"
	local verb = "Earn"
	if count == 0 then
		verb = "Lose"
	elseif count > 1 then
		unit ..= "s"
	end
	return { Count = count, Label = verb .. " " .. count .. " " .. unit .. "!" }
end

local levels = {
	{ id = 1, rewards = { reward(1), { Count = 5, Label = "Gem" } } },
	{ id = 2, rewards = { { Count = 2, Label = "Key" }, reward(25) } },
	{ id = 3, rewards = { reward(0) } },
}
local total = 0
for _, level in ipairs(levels) do
	for i, r in ipairs(level.rewards) do
		total += r.Count
		print(level.id, i, r.Count, r.Label)
	end
end
print(total)
