-- `not ready(item.next == nil)` with `ready` copied in: the `not` belongs to
-- what the call returns, not to the comparison passed to it.
local log = {}
local function ready(flag)
	if flag then
		return #log < 3
	end
	log[#log + 1] = "waited"
end

local function step(item)
	if not ready(item.next == nil) then
		return "busy"
	end
	if not ready(item.count ~= 0) then
		return "empty"
	end
	return "ok"
end

for _, item in {{}, {next = 1}, {count = 0}, {next = 2, count = 5}, {}} do
	print(step(item), #log)
end
