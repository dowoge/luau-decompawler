-- The script reads the global `_` inside loops whose key it never uses; that
-- key must not be renamed `_`, or the reads see the key instead of the global.
_ = "shared"
local ranks = { "bronze", "silver" }
for slot, rank in ipairs(ranks) do
	print(rank, _)
end
local lookup = { gold = 3 }
for name, value in pairs(lookup) do
	print(value, _ .. "!")
end
local function roll() return 4 end
local unused = roll()
print(_)
