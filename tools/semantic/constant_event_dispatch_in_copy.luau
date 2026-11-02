-- A logging helper copied with a constant event kind and cause: its comparisons on the
-- kind and its `or` default are decided per copy, with the same effects as the call.
local live = select("#", ...) == 0
local ledger = {}
local stamps = 0
local function stampNow()
	stamps += 1
	return "t" .. stamps
end

local function note(id, kind, why)
	if not live then
		return
	end
	local mark = stampNow()
	local entry = ledger[id]
	if entry == nil then
		entry = { first = why or "n/a", kind = kind, hits = 0 }
		ledger[id] = entry
	end
	if kind == "OPEN" then
		entry.opened = mark
	elseif kind == "CLOSE" then
		entry.cause = why
	elseif kind == "HIT" then
		entry.hits += 2
	end
end

function process(id, ok)
	if ok then
		note(id, "HIT", "sword")
	else
		note(id, "CLOSE", "timeout")
	end
	note(id, "OPEN")
	local e = ledger[id]
	return e.first, e.kind, e.hits, e.cause, e.opened
end
for _, step in ipairs({ { "gate", true }, { "door", false }, { "gate", false }, { "gate", true }, { "hatch", true } }) do
	print(process(step[1], step[2]))
end
print(stamps)
