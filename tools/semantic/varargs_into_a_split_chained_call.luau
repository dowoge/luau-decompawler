-- `return t[1]({ ... })()(...)` with a test inside the table's fields: the
-- expression is written as statements, the callee a local of its own. The
-- `...` passed to the last call went into a local of its own instead
-- (`local v = ...` then `return f()`), and the call got nothing.

local cache = {}
local function make(t)
	return function()
		return function(...)
			print(t[1], t[2], t[3], ...)
			return select("#", ...)
		end
	end
end
local function run(t, ...)
	return t[1]({ t[2], cache.k or (function(...)
		cache.k = select("#", ...)
		return cache.k
	end)(1, 2), t[3] })()(...)
end
print(run({ make, "a", "b" }, 7, 8, 9))
print(run({ make, "c", "d" }, 7))
print(run({ make, "e", "f" }))
