-- Constructor fields are read before later list calls run, and a store after a
-- template may join it only ahead of inert entries, never ahead of other calls.
local level = 1
local service = {}
function service:Require(name)
	level = 2
	return name
end
local t = { stamp = level, service:Require("Combat"), 0 }
print(t.stamp, level, t[1], t[2])

local order = {}
local Loader = {}
function Loader:boot()
	table.insert(order, "boot")
	return 10
end
function Loader:depth()
	table.insert(order, "depth")
	return 1
end
function Loader:width()
	table.insert(order, "width")
	return 2
end
local cfg = { width = 1, height = 2 }
local n = Loader:boot()
cfg.depth = Loader:depth()
cfg.width = Loader:width()
print(table.concat(order, ","), n, cfg.width, cfg.height, cfg.depth)
