-- Curried UI factories with a computed property key and an open last child,
-- failing stores and partial tables keep their evaluation order and values.
local trace = {}
local function step(label, value)
	table.insert(trace, label)
	return value
end

local ui = {}
function ui:make(class)
	step("make:" .. class)
	return function(props)
		step("build:" .. class)
		return props, false, "end:" .. class
	end, "ignored"
end

local CHILDREN = {}
local function childrenKey()
	return step("children", CHILDREN)
end

local tree = {
	step("head", 1),
	Title = step("title", 2),
	ui:make("Panel")({
		Width = step("width", 3),
		[childrenKey()] = { (ui:make("Label")({ Text = step("text", 4) })) },
	}),
}
print(tree[1], tree.Title, tree[2].Width, tree[2][CHILDREN][1].Text, tree[3], tree[4])
print("order", table.concat(trace, ","))

local currentKey = "early"
local function rename()
	currentKey = "late"
end
local bag = {}
local frozen = currentKey
rename()
bag[frozen] = 1
print("frozen", bag.early, bag.late)

local function failingStore(bad)
	trace = {}
	local ok = pcall(function()
		local t = { step("first", 1) }
		t[step("key", bad)] = step("value", 2)
		t.After = step("after", 3)
		return t
	end)
	print("failed", ok, table.concat(trace, ","))
end
failingStore(nil)
failingStore(0 / 0)

trace = {}
local refilled = { primary = nil, secondary = step("secondary", 2) }
refilled.primary = step("primary", 1)
print("refilled", refilled.primary, refilled.secondary, table.concat(trace, ","))

trace = {}
local maker = ui:make("Hidden")
local enabled = false
local built = enabled and maker({})
print("skipped", built, table.concat(trace, ","))

local partial
local function inspectPartial()
	print("partial", partial.first, partial.last)
	return 2
end
partial = { first = 1 }
partial.last = inspectPartial()
print("complete", partial.first, partial.last)
