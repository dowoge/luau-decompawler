-- Tiny helpers copied beside hand-written code that computes the same thing: every copy
-- and every look-alike keeps its own value and its effects in the same order.
local trace = {}
local function socket(name)
	return {
		Close = function()
			trace[#trace + 1] = "close " .. name
		end,
	}
end

local function closeIf(s)
	if s ~= nil and s.Close then
		s:Close()
	end
end

local function blank(t)
	return #t == 0 and next(t) == nil
end

local function towards(from, to, alpha)
	return from * (1 - alpha) + to * alpha
end

local function weave(a1, a2, a3, b1, b2, b3, c1, c2, c3)
	return a1 * b2 * c3 + a2 * b3 * c1 + a3 * b1 * c2 - a3 * b2 * c1
end

local function absorb(pool, tag, list)
	for _, v in ipairs(list) do
		pool.size += 1
		trace[#trace + 1] = tag .. v
	end
	if pool.size > 2 then
		trace[#trace + 1] = "full"
	end
	return pool
end

local function forward(pool, tag, list)
	absorb(pool, tag, list)
end

local engine = { revs = 0 }
function engine:Running()
	self.revs += 1
	return self.revs % 3 == 0
end
local function spinning()
	return engine:Running()
end

local function succ(n)
	return n + 1
end

local function before(x, y, asc)
	return if asc then x < y else x > y
end

local function sortOn(key, asc)
	return {
		Key = key,
		Less = function(p, q)
			return before(p[key] or 0, q[key] or 0, asc)
		end,
	}
end

local function share(a, b)
	return a / (a + b + 1)
end

local function shutdown(conns)
	closeIf(conns.main)
	closeIf(conns.backup)
	closeIf(conns.debug)
	local handmade = conns.extra
	if handmade ~= nil and handmade.Close then
		handmade:Close()
	end
	return blank(conns.pending) and "drained" or "busy"
end

local function mixPair(p, q, t)
	local a = towards(p, q, t)
	local b = towards(q, p, t)
	return a, math.clamp(b, 0, 10), p * (1 - t) + q * t
end

local function invert(m)
	local det = weave(m[1], m[2], m[3], m[4], m[5], m[6], m[7], m[8], m[9])
	if det == 0 then
		return "flat"
	end
	return weave(m[9], m[8], m[7], m[6], m[5], m[4], m[3], m[2], m[1]) / det
end

local function fill(pool)
	forward(pool, "a", { 1, 2 })
	absorb(pool, "b", { 3 })
	forward(pool, "c", {})
	return pool.size
end

local function idleState()
	if spinning() then
		return "spin"
	end
	return spinning() and "spin" or "rest"
end

local function bumpBoth(n, m)
	return succ(n) * 10 + (m + 1)
end

local function sorters()
	local s = {}
	s.ByName = sortOn("Name", true)
	s.ByAge = sortOn("Age", false)
	s.ByRank = sortOn("Rank", true)
	return s
end

local function split(a, b, flip)
	return (flip and share(a, b) or share(b, a)) * 2, share(a, b) - b / (a + b + 1)
end

local function decay(cfg, t)
	local start = cfg.Start or 1
	local level = (start * (1 - t) + (cfg.End or 0) * t) * 2
	local half = cfg.Half or 5
	if cfg.Target then
		half = towards(half, cfg.Target, t)
	end
	return level, half
end

print(shutdown({ main = socket("main"), debug = socket("debug"), extra = socket("extra"), pending = {} }))
print(shutdown({ pending = { 1 } }), table.concat(trace, ","))
print(mixPair(2, 6, 0.25))
print(invert({ 2, 0, 1, 1, 3, 0, 0, 1, 4 }), invert({ 0, 0, 0, 1, 1, 1, 2, 2, 2 }))
local pool = { size = 0 }
print(fill(pool), table.concat(trace, ","))
print(idleState(), idleState(), idleState(), idleState())
print(bumpBoth(4, 5))
local s = sorters()
print(s.ByName.Less({ Name = 1 }, { Name = 2 }), s.ByAge.Less({ Age = 1 }, { Age = 2 }), s.ByName.Less == s.ByRank.Less, s.ByAge.Key)
print(split(1, 3, true), split(1, 3, false))
print(decay({ Start = 3, End = 9, Half = 12, Target = 20 }, 0.5))
print(decay({ End = 2 }, 0.75))
