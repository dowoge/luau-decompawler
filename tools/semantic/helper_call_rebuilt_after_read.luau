-- An inlined helper's leading statement writes a global or a table field that
-- the same statement reads first; the rebuilt call must not move past that read.
function setMode(m)
	Mode = m
end
local function apply(m)
	setMode(m)
	if m then
		return "on"
	end
	return "off"
end
for _, m in ipairs({ "night", false }) do
	local label = apply(m)
	print(Mode, label)
end

local held = {}
local function store(m)
	held.mode = m
	if m then
		return "on"
	end
	return "off"
end
for _, m in ipairs({ "day", false }) do
	local label = store(m)
	print(held.mode, label)
end
