-- Small helpers inlined at both call sites keep one closure constant per literal for both
-- copies: the stored callbacks compare equal unless they capture a parameter.
local ACCENT = "teal"
local function style(frame)
	frame.OnHover = function()
		return "hover"
	end
	frame.GetAccent = function()
		return ACCENT
	end
	return frame
end
local function bindOwner(frame, owner)
	frame.GetOwner = function()
		return owner
	end
	return frame
end
local function addBlink(frame)
	local function blink(n)
		if n <= 0 then
			return "lit"
		end
		return blink(n - 1)
	end
	frame.Blink = blink
	return frame
end
local a = addBlink(bindOwner(style({}), "p1"))
local b = addBlink(bindOwner(style({}), "p1"))
print(a.OnHover == b.OnHover, a.GetAccent == b.GetAccent, a.GetOwner == b.GetOwner, a.Blink == b.Blink)
print(a.OnHover(), b.GetAccent(), a.GetOwner(), b.Blink(3))
