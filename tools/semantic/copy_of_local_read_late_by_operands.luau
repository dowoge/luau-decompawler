-- a copy of a local that a copied-in helper changes, read after the helper
-- runs: by a comparison, an arithmetic operand, a receiver. Luau reads a
-- local operand where it is used, so the copy stays a local of its own.
local function run(start)
	local log = {}
	local x = start
	local function bump()
		x += 9
		return 5
	end
	local before = x
	log[#log + 1] = tostring(bump() < before)
	before = x
	log[#log + 1] = tostring(before - bump())
	before = x
	log[#log + 1] = tostring(bump() + before)
	local host = { name = "first" }
	function host:show(n)
		return self.name .. n
	end
	local function swap()
		host = { name = "second", show = host.show }
		return 1
	end
	local target = host
	log[#log + 1] = target:show(swap())
	return table.concat(log, " "), x, host.name
end
print(run(1))
print(run(7))
