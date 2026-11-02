-- A local captured and never written again (a copy, a swap source, a select
-- result) is its own variable: closures over it ignore later writes elsewhere.
local function freezeAfter(start)
	local hp = start
	local function live()
		return hp
	end
	hp = hp * 2
	local snapshot = hp
	local function both()
		return snapshot, hp
	end
	hp = hp + 1
	local s, h = both()
	return s, h, live()
end
print(freezeAfter(5))

local function pairUp(left, right, flip)
	local keep = right
	local function view()
		return keep
	end
	if flip then
		left, right = right, left
	end
	right = right .. "!"
	return view(), left, right
end
print(pairUp("L", "R", true))
print(pairUp("L", "R", false))

local function chosen(a, b, useA)
	local pick = if useA then a else b
	local function get()
		return pick
	end
	a, b = b * 10, a * 10
	return get(), a, b
end
print(chosen(1, 2, true))
print(chosen(1, 2, false))

local function account(balance, fee, waive)
	local function preview()
		return balance - (if waive then 0 else fee)
	end
	local function startingFee()
		return fee
	end
	local spare = fee
	balance, fee = balance * 2, spare + 1
	return preview(), startingFee(), balance, fee
end
print(account(100, 5, false))
print(account(100, 5, true))

local function signed(input, cell)
	local function current()
		return math.min(cell, input)
	end
	local function initial()
		return input
	end
	cell = math.ceil(-0.5)
	return current(), initial(), 1 / cell
end
print(signed(7, 2))
print(signed(-7, 2))
