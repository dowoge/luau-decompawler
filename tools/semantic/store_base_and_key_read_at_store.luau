-- A store reads a register base and key after its value, and a compound update
-- reads its base once; snapshots of locals a callback rebinds keep the old ones.
local function keyedStore(t)
	local index = 1
	local function advance()
		index = 2
		return "filled"
	end
	local held = index
	t[held] = advance()
	return t[1], t[2], index
end
print(keyedStore({}))

local Left, Right = {}, {}
local function baseStore()
	local side = Left
	local function flip()
		side = Right
		return 42
	end
	local held = side
	held.value = flip()
	return Left.value, Right.value
end
print(baseStore())

local function pairStore(t, u)
	local index = 1
	local function advance()
		index = 2
		return "a"
	end
	local held = index
	t[held], u.tag = advance(), "b"
	return t[1], t[2], u.tag
end
print(pairStore({}, {}))

local function directStore()
	local box = {}
	local spare = {}
	local function swap()
		box = spare
		return 5
	end
	local v = swap()
	box.size = v
	return box == spare, spare.size
end
print(directStore())

local function upvalueTarget()
	local box = {}
	local spare = {}
	local function swap()
		box = spare
		return 6
	end
	return function()
		local target = box
		local v = swap()
		target.size = v
		return target == spare, spare.size, target.size
	end
end
print(upvalueTarget()())

local function makeTimer(ticks)
	local timer = { Left = ticks }
	return function()
		timer.Left -= 1
		if timer.Left == 0 then
			timer = nil
			return "done"
		end
		return timer.Left
	end
end
local tick = makeTimer(2)
print(tick(), tick())

local crate = { Weight = 4 }
local handle = crate
handle.Weight *= 5
print(crate.Weight)

local cart = { Items = 1 }
local function restock()
	cart = { Items = 50 }
	return 3
end
local first = cart
first.Items += restock()
print(first.Items, cart.Items)
