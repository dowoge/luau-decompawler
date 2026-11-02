-- Table constructors of many shapes, some never read: unused ones still run the calls
-- inside them, and every table that is read keeps all of its entries.
local built = {}
local function mark(s)
	built[#built + 1] = s
	return s
end

local PALETTE = { Primary = "#ff8800", Border = 2 }
local SIZES = { 8, 12, [6] = 20 }
local Builders = {}
Builders.Label = function(text)
	return { Text = text }
end
local spare = {}
local tagged = { stamp = mark("palette") }
local lonely = {}
lonely.flag = mark("flag")

local Widgets = {}
function Widgets.measure(w)
	local tmp = { w, w + 1, mark("measure") }
	return w * 4
end

local function dump(value)
	if type(value) ~= "table" then
		return tostring(value)
	end
	local keys = {}
	for k in pairs(value) do
		keys[#keys + 1] = k
	end
	table.sort(keys, function(a, b)
		return tostring(a) < tostring(b)
	end)
	local parts = {}
	for _, k in ipairs(keys) do
		parts[#parts + 1] = tostring(k) .. ":" .. dump(value[k])
	end
	return "{" .. table.concat(parts, ",") .. "}"
end

print(Widgets.measure(3), table.concat(built, "/"))
local samples = {
	{ Icon = "rbxassetid://5", Size = 24 },
	{ 10, 20, 30, 40, 50, 60, 70, 80 },
	{ Ref = PALETTE.Primary },
	{ Width = math.max(3, 7), Holder = {}, Shift = -2.25 },
	{ "first", "second", key = "third" },
	{ grid = { 1, 2, 3 } },
	{ Note = "a sentence that runs on long enough to need its own line in any layout", Id = 7 },
}
for i, s in ipairs(samples) do
	print(i, dump(s))
end
