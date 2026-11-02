-- Arguments that have effects, passed to inlined helpers: each runs exactly once and in
-- the order written, whatever order the helper's body reads its parameters in.
local dealt = {}
local Board = {}
function Board:place(owner, card, extra)
	dealt[#dealt + 1] = card
	return { owner = owner, card = card, extra = extra }
end

local function dealRound(hand, deck)
	local function give(card, extra)
		local slot = Board:place(hand, card, extra)
		hand.last = slot
		slot.face = "up"
	end
	give(table.remove(deck), { bonus = table.remove(deck) })
	local a = hand.last.card .. "+" .. hand.last.extra.bonus
	give(#deck .. "left", { bonus = table.remove(deck, 1) })
	local b = hand.last.card .. "+" .. hand.last.extra.bonus
	give(table.remove(deck, 1), { bonus = #deck })
	return a, b, hand.last.card .. "+" .. hand.last.extra.bonus
end
print(dealRound({}, { "2H", "3C", "QD", "KS", "AS" }))
print(table.concat(dealt, ","))

local order = {}
local Probe = {}
function Probe.mark(tag, value)
	order[#order + 1] = tag
	return value
end
local mark = Probe.mark

local function crossed(card, extra)
	local slot = Board:place(order, extra, card)
	order[#order + 1] = "x:" .. tostring(slot.card) .. ":" .. tostring(slot.extra)
end

local function mirrored(card)
	local slot = Board:place(order, card, card)
	order[#order + 1] = "m:" .. tostring(slot.extra)
end

local function onlyWhen(cond, card)
	if cond then
		order[#order + 1] = "w:" .. tostring(card)
	end
	order[#order + 1] = "done"
end

onlyWhen(#order == 0, mark("lead", "A"))
crossed(mark("c1", "Q"), mark("c2", "K"))
mirrored(mark("m", 7))
onlyWhen(#order > 100, mark("never", "Z"))
print(table.concat(order, " "))
