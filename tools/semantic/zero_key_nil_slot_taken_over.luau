-- `{[0] = nil, owner = ...}` sizes two hash nodes; whether `owner` reuses the
-- nil node decides whether a later store through `[-0]` keeps the key +0.
local function lookup()
	return "worker"
end

local function withOwner(fetch)
	local t = { [0] = nil, owner = fetch() }
	t[-0] = 7
	return t
end

local function withRank(fetch)
	local t = { [0] = nil, rank = fetch() }
	t[-0] = 9
	return t
end

for _, build in { withOwner, withRank } do
	for key, value in pairs(build(lookup)) do
		if type(key) == "number" then
			print("zero", 1 / key, value)
		else
			print(key, value)
		end
	end
end
