-- A helper that returns its parameter unread, copied in: the argument is
-- made where the result goes, just before the copy, and the copy does not
-- touch it. Once the call came back with no argument (`counted()`) beside a
-- local holding it. Near misses: the argument a local or a constant, a value
-- made just before that is not the argument, the result unused.

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

local function three(n)
	return { counted(n), counted(n * 2), counted(n * 3) }
end

local function near(n)
	local a = n * 7
	local b = counted(5)
	counted(n + 2)
	local c = { n - 1, counted(a), counted(n .. "!") }
	return a, b, c[1], c[2], c[3], #c
end

local log = {}
local function noted(v)
	log[#log + 1] = v
	return v
end

local function chain(n)
	local x = noted(n + 1)
	local y = noted(x * 2)
	return x + y, noted(y - x)
end

print(pick(5)[2], #three(2), three(4)[3])
print(near(3))
print(chain(10))
print(calls, #log, log[1], log[2], log[3])
