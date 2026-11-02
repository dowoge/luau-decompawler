-- `debug.info` reveals which function is running: code inlined from a helper must not turn
-- back into a call of it, and a function fetched through it may write a captured local.
local Inspect = {}
local function runningIs(fn)
	return debug.info(1, "f") == fn
end
local chunkFn = debug.info(1, "f")
local check = debug.info(1, "f") == chunkFn
Inspect.self = check
print("self", check)

local function whoCalled(...)
	return debug.info(2, "f")
end
local function calledFrom(fn)
	return (whoCalled() == fn) and "direct" or "elsewhere"
end
print("caller", (whoCalled() == chunkFn) and "direct" or "elsewhere")

local energy = 1
local captured
local function recharge(...)
	captured = debug.info(1, "f")
	energy = 9
	return 0
end
recharge()
energy = 1
local function later()
	return energy + captured()
end
local now = energy + captured()
print("energy", now, energy)
return runningIs, calledFrom, later, Inspect
