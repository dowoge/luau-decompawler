-- Helpers copied with constant flags, so each copy keeps one branch, beside code written
-- by hand that does the same: every copy behaves like the call it came from.
local journal = {}
local hud = { title = "Lobby", hint = 1 }
local function beep()
	journal[#journal + 1] = "beep"
end

local function present(card)
	if not card then
		beep()
		hud.title, hud.hint = nil, nil
		return false
	end
	hud.title = card.title
	hud.hint = card.hint
	journal[#journal + 1] = "card:" .. card.hint
	return true
end

local function settle(part, purge)
	if not purge then
		part.Frozen = true
		part.Ghost = false
		return
	end
	table.clear(part)
end

local function enterArea(name)
	present({ title = name, hint = #name })
	return hud.title
end
local function leaveArea()
	present(nil)
	return hud.title
end
local function fadeOut(now)
	if now then
		beep()
		hud.title, hud.hint = nil, nil
	end
	return hud.hint
end
local states = {
	enterArea("Cave"),
	leaveArea(),
	enterArea("Forest"),
	fadeOut(false),
	fadeOut(true),
}
print(states[1], states[2], states[3], states[4], states[5], table.concat(journal, ","))

local purged = { 1, 2, a = 3 }
settle(purged, true)
local frozen = {}
settle(frozen, false)
local manual = {}
manual.Frozen = true
manual.Ghost = false
print(next(purged), frozen.Frozen, frozen.Ghost, manual.Ghost)

local function clone(t, deep)
	if not deep then
		return table.clone(t)
	end
	local function walk(node)
		local copy = table.clone(node)
		for k, v in pairs(copy) do
			if type(v) == "table" then
				copy[k] = walk(v)
			end
		end
		return copy
	end
	return walk(t)
end

local stage = { left = 0, right = 0, gaps = 0 }
local function arrange(compact)
	if not compact then
		stage.left = 20
		stage.right = stage.left * 2
		stage.gaps += 1
	else
		stage.left, stage.right = 5, 5
	end
end

local function decorate(sign, lit)
	sign.Dim = if lit then 0 else 0.8
	if lit then
		sign.Neon = true
	else
		sign.Neon = false
		sign.Dim += 0.05
	end
end

local function token()
	local function id()
		return "tok"
	end
	return id
end

local function inventory(src, extras)
	local mine = clone(src)
	for k, v in pairs(extras) do
		if mine[k] == nil and type(v) == "table" then
			mine[k] = clone(v, true)
		end
	end
	return mine, clone(extras, true)
end

local function showroom(power)
	arrange(false)
	local sign = {}
	decorate(sign, power > 3)
	return stage.left + stage.right + stage.gaps, sign.Neon, sign.Dim
end

local mine, copyAll = inventory({ sword = 1 }, { shield = { layers = { 2, 3 } }, sword = 9 })
print(mine.sword, mine.shield.layers[2], mine.shield ~= copyAll.shield, copyAll.shield.layers ~= mine.shield.layers)
print(showroom(5))
print(showroom(1))
print(token() == token(), token()())

local function rank(a, b, highFirst)
	if a == b then
		return 0
	end
	if highFirst then
		return if a > b then -1 else 1
	end
	return if a < b then -1 else 1
end

local boards = {
	Score = function(p, q)
		return rank(p.score, q.score, true)
	end,
	Time = function(p, q)
		return rank(p.time or 99, q.time or 99, true)
	end,
	Name = function(p, q, high)
		return rank(p.name, q.name, high)
	end,
}
local p1, p2 = { score = 10, time = 30, name = "ann" }, { score = 4, name = "bob" }
print(boards.Score(p1, p2), boards.Time(p1, p2), boards.Name(p1, p2, false), boards.Name(p2, p2, true), boards.Score(p2, p1))
