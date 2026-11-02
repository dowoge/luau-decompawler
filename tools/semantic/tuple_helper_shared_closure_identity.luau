-- Multi-value helpers handing out capture-free function literals, copied more than once:
-- whether two results are one and the same function object must not change.
local function stamp()
	return function()
		return "s"
	end
end

local function bundle()
	local mine = function()
		return "m"
	end
	return stamp(), mine, stamp()
end

local function withSize()
	local function sizer()
		return 3
	end
	return 4, sizer
end

local function pop2(q)
	local a = table.remove(q, 1)
	q.pops = (q.pops or 0) + 1
	return a, q[1]
end

local x1, m1, y1 = bundle()
local x2, m2, y2 = bundle()
local same = { x1 == y1, x1 == x2, y1 == y2, m1 == m2, m1 == x1 }
for i, v in ipairs(same) do
	same[i] = tostring(v)
end
print(table.concat(same, " "), x1() .. m2() .. y2())
local n1, s1 = withSize()
local n2, s2 = withSize()
print(s1 == s2, n1 + n2 + s1() + s2())
local q = { "a", "b", "c" }
local p, r = pop2(q)
local t, u = pop2(q)
print(p, r, t, u, q.pops, #q)
