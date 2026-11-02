-- Function literals whose code matches a helper's body: whether each literal is a single
-- shared object or a fresh one per run depends on its captures, and must not change.
local target = nil
local function describeTarget()
	return target and target.name or "nobody"
end
local function setTarget(t)
	target = t
end

local watchers = {}
for i = 1, 2 do
	watchers[i] = function()
		return target and target.name or "nobody"
	end
end
setTarget({ name = "slime" })
print(watchers[1] == watchers[2], describeTarget(), watchers[1](), watchers[2]())

local function workshop(level)
	local function polish(item)
		item.shine = item.shine + level
		item.value = item.shine * 3
		return level
	end
	local made = { polish({ shine = 1 }), polish({ shine = 2 }) }
	return function(item)
		item.shine = item.shine + 1
		item.value = item.shine * 3
		return item.value
	end, made[1] + made[2]
end
local f1, n1 = workshop(1)
local f2, n2 = workshop(5)
print(f1 == f2, n1, n2, f1({ shine = 0 }), f2({ shine = 4 }))

local clicks = 0
local function counterFactory()
	return function()
		clicks += 1
		return (target and target.name or "nobody") .. clicks
	end
end
local c1, c2 = counterFactory(), counterFactory()
print(c1 == c2, c1(), c2())
