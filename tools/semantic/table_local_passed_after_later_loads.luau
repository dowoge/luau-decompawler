-- A table built into its own local and passed to a call made after it: the
-- table's constructor runs before what the call loads (the callee, its
-- object, upvalues), so they must be read after it, not hoisted above it.
-- A table assigned to an existing local is that local's new value instead.

local obj = {
	m = function(_, t)
		print("old method", t[1], t.k)
	end,
}

local function replace()
	obj = {
		m = function(_, t)
			print("new method", t[1], t.k)
		end,
	}
	return 1
end

local function run()
	local t = { replace(), k = "first" }
	obj:m(t)
end
run()

local calls = 0
local function counted(v)
	calls += 1
	return v
end

local function pick(n)
	local t = { counted(n), counted(n + 1) }
	print(calls, #t, t[1], t[2])
	return t
end
local r = pick(5)
print(calls, r[2])

local handler = print
local function swap()
	handler = function(...)
		print("swapped", ...)
	end
	return "v"
end
local m = { swap() }
handler(m[1], #m)

-- an existing local given a new table each time round
local node = { depth = 0 }
for i = 1, 3 do
	node = { depth = node.depth + 1, parent = node, i = i }
end
print(node.depth, node.parent.depth, node.parent.parent.i)

-- several tables, each a local, used in another order
local a = { 1 }
local b = { 2, a }
local c = { b, a, 3 }
print(c[3], c[1][1], c[1][2][1], c[2][1], #c)
