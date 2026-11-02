-- `[0]` and `[-0]` name one slot: a store through either spelling updates the
-- entry the other made, and the key keeps the sign it was created with.
local function overwrite()
	local t = { [0] = "a", [-0] = "b" }
	t[0] = "c"
	return t[0], t[-0]
end
print(overwrite())

local function viaNegative(k)
	local t = { [0] = 1 }
	t[-0] = 2
	return t[0], t[k]
end
print(viaNegative(0), viaNegative(-0))

local function fetch()
	return "tag"
end
local function kept(fetch)
	local t = { [0] = 0, label = fetch() }
	t[-0] = 30
	return t
end
local function negativeFirst(fetch)
	local t = { [-0] = 5, label = fetch() }
	t[0] = 6
	return t
end
for _, build in { kept, negativeFirst } do
	for key, value in pairs(build(fetch)) do
		if type(key) == "number" then
			print("zero", 1 / key, value)
		end
	end
end
