-- An upvalue operand is read before a later call operand runs; inlined, the same operand
-- is a register read when the store or operation happens. Both orders must survive.
local function via(fn)
	return fn()
end

local function slots(first, second)
	local key = first
	local box = {}
	local function moveKey()
		key = second
		return "k"
	end
	local function fillCtor()
		return { [key] = via(moveKey) }
	end
	local built = fillCtor()
	key = first
	local function fillStore()
		box[key] = via(moveKey)
	end
	fillStore()
	key = first
	local function fillPinned()
		local pinned = key
		box[pinned] = via(moveKey) .. "!"
	end
	fillPinned()
	return built[first], built[second], box[first], box[second]
end
print("slots", pcall(slots, "north", "south"))

local function receivers()
	local bagA, bagB = { tag = "A" }, { tag = "B" }
	local target, field = bagA, "count"
	local function retarget()
		target = bagB
		field = "spill"
		return 5
	end
	local function store()
		target[field] = via(retarget)
	end
	store()
	target = bagA
	local function retargetRead()
		target = bagB
		return "tag"
	end
	local function readThrough()
		return target[via(retargetRead)]
	end
	return bagA.count, bagA.spill, bagB.count, bagB.spill, readThrough()
end
print("receivers", pcall(receivers))

local function gauges(start, after)
	local level = start
	local function spike()
		level = after
		return 7
	end
	local function plus()
		return level + via(spike)
	end
	local sum = plus()
	level = start
	local function below()
		return level < via(spike)
	end
	local less = below()
	level = start
	local function same()
		return level == via(spike)
	end
	local equal = same()
	level = start
	local function peak()
		return math.max(level, (via(spike)))
	end
	local top = peak()
	level = start
	local biggest = math.max
	local function peakAlias()
		return biggest(level, (via(spike)))
	end
	return sum, less, equal, top, peakAlias()
end
print("gauges", pcall(gauges, 3, 20))

local function speakers()
	local function say(self, suffix)
		return self.voice .. suffix
	end
	local npc = { voice = "hi", say = say }
	local boss = { voice = "ROAR", say = say }
	local function swapSpeaker()
		npc = boss
		return "?"
	end
	local function talk()
		return npc:say(via(swapSpeaker))
	end
	return talk()
end
print("speakers", pcall(speakers))

local primary, backup = {}, {}
local active = primary
local function failover()
	active = backup
	return 1
end
local function writeVia(t)
	t.hits = failover()
end
active.hits = failover()
print("chunk", primary.hits, backup.hits)

local scores = {}
local slot = "a"
local function bumpSlot()
	slot = "b"
	return 2
end
local function writeAt(k)
	scores[k] = bumpSlot()
end
scores[slot] = bumpSlot()
print("chunk", scores.a, scores.b)
return writeVia, writeAt
