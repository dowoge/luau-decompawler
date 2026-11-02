-- A helper defined inside a loop body reads that pass's loop value: its copies later in
-- the same pass read that value, never another pass's.
function weigh(crates)
	local totals = {}
	for _, crate in ipairs(crates) do
		local function part(slot)
			local v = crate[slot]
			if v == nil then
				return 0
			end
			return v * crate.mult
		end
		totals[#totals + 1] = part(1) - part(2) + part(3) * part(1)
	end
	return totals
end

local out = weigh({ { 4, 1, 2, mult = 1 }, { 7, 3, mult = 2 }, { 1, 0, 5, mult = 3 } })
print(out[1], out[2], out[3])

function ladder(n)
	local rungs = {}
	local readers = {}
	for i = 1, n do
		local function height(step)
			return i * 10 + step
		end
		rungs[i] = height(1) + height(2)
		readers[i] = height
	end
	return table.concat(rungs, ","), readers[1](0), readers[n](0)
end
print(ladder(4))
