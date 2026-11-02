-- A store after a constructor ending in `...` or an open call must not join it
-- behind that item, where the open item would be cut to a single value.
local function pack(...)
	local t = { ... }
	t.n = select("#", ...)
	return t
end

local function tagged(f)
	local t = { "head", f() }
	t.tag = "z"
	return t
end

local function three()
	return 1, 2, 3
end

local function compose(seed, flip, ...)
	local sign = if flip then 1 else -1
	local t = {
		dir = sign,
		[2] = if flip then seed else seed * 3,
		"head",
		sign,
		...,
	}
	t[-0] = 0.25
	return t
end

local p = pack("a", "b", "c")
print(p.n, #p, p[1], p[2], p[3])
local q = tagged(three)
print(#q, q[1], q[2], q[3], q[4], q.tag)
local t = compose(2, true, "v", "w", "x")
print(#t, t[1], t[2], t[3], t[4], t[5], t[-0])
