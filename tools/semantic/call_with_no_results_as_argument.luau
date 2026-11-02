-- A call returning nothing passes nil once cut to one value; left open as a
-- builtin's last argument it passes nothing, which builtins notice.
local Inventory = {}
function Inventory.slotOf(items, wanted)
	for i, item in ipairs(items) do
		if item == wanted then
			return i
		end
	end
end
local function describe(items, wanted)
	local slot = Inventory.slotOf(items, wanted)
	return tostring(slot)
end
print(pcall(describe, { "axe", "rope" }, "lamp"))
print(pcall(describe, { "axe", "rope" }, "rope"))

local function silent() end
local function trio()
	return 7, 8, 9
end

local function counts(produce)
	return select("#", (produce())), select("#", produce())
end
print(counts(silent))
print(counts(trio))

local function biggest(produce)
	return math.max(1, (produce() or 0)), (pcall(math.max, 1, (produce())))
end
print(biggest(silent))
print(biggest(trio))

local function appendOne(list, produce)
	table.insert(list, (produce()))
	return #list, list[#list]
end
print(appendOne({ "x" }, trio))
print(pcall(appendOne, { "x" }, silent))

local function clearSlot(t, produce)
	rawset(t, "slot", (produce()))
	return t.slot
end
print(clearSlot({ slot = 5 }, silent))

local function dropMeta(t, produce)
	setmetatable(t, (produce()))
	return getmetatable(t) == nil
end
print(dropMeta(setmetatable({}, {}), silent))

local function head()
	local function both()
		return 6, 7
	end
	return both, (both())
end
print(select("#", head()), select(2, head()))
