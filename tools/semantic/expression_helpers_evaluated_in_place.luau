-- Helpers whose whole body is one returned expression, copied into stores, receivers,
-- constructors and arguments: values, evaluation order and result counts all hold.
local scaleBy = { k = 1.5 }
function scaleBy:apply(lo, hi)
	return (hi - lo) * self.k
end
local swing = 2

local function boost(v)
	return v and (v + scaleBy:apply(-swing, swing) or v)
end

local Session = { Current = { Player = { Tag = "p1", Lives = 3 } } }
local active = Session.Current
local function player()
	return active and active.Player
end

local function initials(words)
	return table.concat(words, ".", 1, 2)
end

local function triple(s)
	return string.rep(s, 3, "-")
end

local ticks = 0
local meter = {}
function meter.next()
	ticks += 2
	return ticks
end

local function scenario(opts, words, list, card)
	opts.Jump = boost(opts.Jump)
	list[#list + 1] = player()
	card.Short = initials(words[1])
	opts.Run.Speed = boost(opts.Run.Speed)
	local snapshot = { owner = player(), short = initials(words[2]):upper() }
	return opts.Jump, opts.Run.Speed, snapshot.owner.Tag, snapshot.short, (player() or {}).Lives, initials(words[3]), #list
end

local card = {}
print(scenario({ Jump = 10, Run = { Speed = 4 } }, { { "a", "b" }, { "c", "d" }, { "e", "f", "g" } }, { "x" }, card))
print(card.Short, scenario({ Run = {} }, { { "p", "q" }, { "r", "s" }, { "t", "u" } }, {}, card))

local function measured()
	local m = meter.next()
	return ticks, m and (m + scaleBy:apply(-swing, swing) or m)
end
local function echoed(s, many)
	if many then
		return string.rep(s, 3, "-")
	end
	return s, (string.rep(s, 3, "-"))
end
local function cut(s)
	return (triple(s)), s
end
print(measured())
print(select("#", echoed("o")), echoed("k"), echoed("m", true), select("#", cut("q")), cut("w"))

local served = 0
local function valid(order)
	return order ~= nil and order.items ~= nil and #order.items > 0
end
local function cost(n)
	return n * 4 + math.min(n, 3)
end
local function pair(a, b)
	return "(" .. a .. "," .. b .. ")"
end
local function issue(n)
	served += 1
	return n .. "#" .. served
end

local Kitchen = {}
function Kitchen.accept(order)
	assert(valid(order), "empty order")
	return #order.items
end
function Kitchen.price(a, b)
	local x = cost(a)
	local y = cost(b)
	return pair(x, y)
end
function Kitchen.receipt(n)
	local half = n / 2
	return string.format("%s/%s", half, issue(n))
end
function Kitchen.trace(n)
	local s = issue(n)
	print("served", served, s)
	return s
end

print(Kitchen.accept({ items = { 1, 2 } }), (pcall(Kitchen.accept, { items = {} })))
print(Kitchen.price(2, 5))
print(Kitchen.receipt(9), Kitchen.trace(4), served)
