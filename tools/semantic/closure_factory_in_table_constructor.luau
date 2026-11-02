-- A factory whose returned function captures and rewrites its own parameters, copied
-- into nested constructors with arguments left out: every copy keeps cells of its own.
local function counterFor(label, start, step, suffix)
	return function()
		if start == nil then
			start = 0
		end
		start += step or 1
		if not suffix then
			suffix = "x"
		else
			suffix ..= "!"
		end
		return label .. start .. suffix
	end
end

local function buildMenu()
	return {
		{ name = "plain", next = counterFor("p") },
		{ name = "tuned", next = counterFor("t", 10, 5, "s") },
		extra = { deep = { counterFor("d", nil, 2) } },
	}
end

local menu = buildMenu()
local other = buildMenu()
print(menu[1].next(), menu[1].next(), menu[2].next(), menu[2].next())
print(menu.extra.deep[1](), other.extra.deep[1](), menu.extra.deep[1]())
print(other[2].next(), menu[2].next(), other[1].next())
