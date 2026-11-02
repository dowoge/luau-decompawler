-- Entries stored before a later callback in the same constructor keep the value
-- and identity they had; a bad key raises before the later items run.
local held = 17
local function bump()
	held = 23
	return 1
end
local function none()
	return
end
local function spread()
	return "x", "y"
end

local function early(bump, rest)
	return { [2] = held, [0] = nil, bump(), rest() }
end
local t = early(bump, none)
print(t[1], t[2], held, #t)

local shared = {}
local function first()
	print("first")
	return shared
end
local function pick(flag, a, b)
	return if flag then a else b
end
local function identity(flag, first, pick, rest)
	return { [2] = first(), [0] = nil, pick(flag, "yes", "no"), rest() }
end
local u = identity(false, first, pick, none)
print(u[2] == shared, u[1], #u)

local function runFn(fn)
	return fn()
end

local function captured(value, runFn, rest)
	local function update()
		value = 99
		return true
	end
	local v = { Value = value, [true] = nil, [false] = nil, if runFn(update) then 1 else 2, rest() }
	return v.Value, value, v[1], v[2], v[3]
end
print(captured(17, runFn, spread))

local function keyedCell(key, runFn, rest)
	local function update()
		key = "after"
		return true
	end
	local k = { [key] = 9, [true] = nil, [false] = nil, if runFn(update) then 1 else 2, rest() }
	return k.before, k.after, k[1], #k
end
print(keyedCell("before", runFn, spread))

local calls = 0
local function count()
	calls += 1
	return 3
end
local function keyed(key, count, rest)
	return { [key] = 9, count(), rest() }
end
print((pcall(keyed, nil, count, spread)), calls)
print((pcall(keyed, 0 / 0, count, spread)), calls)
local ok, k = pcall(keyed, "fine", count, spread)
print(ok, k.fine, k[1], k[2], k[3], calls)
