-- A local shared by closures and by inlined helpers that bump it: arguments read it
-- before the bump, closures read it later, and a branch value joins in between.
local function probe(seed, pick)
	local base, counter = seed, seed * 2
	local function bumpAndBase()
		counter += 3
		return base
	end
	local before = bumpAndBase()
	local readers = {}
	for i = 1, 3 do
		readers[i] = function()
			return counter - i
		end
	end
	for _, r in ipairs(readers) do
		print(r())
	end
	local function pass(v)
		counter += 1
		return v
	end
	local after = pass(if pick then counter else seed + 1)
	local last = pass(counter)
	return before, after, last, base, counter, readers[1]()
end
print(probe(5, true))
print(probe(-2, false))
