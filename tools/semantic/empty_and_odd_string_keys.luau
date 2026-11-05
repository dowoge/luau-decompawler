-- A table key that is the empty string, a keyword, or not an identifier is a
-- real key: `t[""]`, `t["end"]`, `t["a b"]`. The empty one once printed as a
-- placeholder (`["?"]`), so the read and the write landed on different keys.
-- Constructor keys, GETTABLEKS reads and SETTABLEKS writes must all agree.

local function build(v)
	local t = {
		[""] = v,
		["end"] = v + 1,
		["a b"] = v + 2,
		["3x"] = v + 3,
		name = v + 4,
	}
	return t
end

local t = build(10)
print(t[""], t["end"], t["a b"], t["3x"], t.name)

-- write through the same keys, then read again
t[""] = 100
t["end"] = 200
t["a b"] = 300
print(t[""], t["end"], t["a b"])

-- the empty key must not collide with any other
local counts = {}
for _, k in { "", "end", "a b", "", "name", "" } do
	counts[k] = (counts[k] or 0) + 1
end
print(counts[""], counts["end"], counts["a b"], counts.name)

-- empty key reached through a helper, inlined into its callers
local function get(store, key)
	return store[key]
end
print(get(t, ""), get(t, "end"), get(build(0), ""))
