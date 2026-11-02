-- -O2 unrolls these two-trip loops (in a function and in the chunk); every copy of the
-- literal loads one shared closure, so the stored handlers compare equal.
local function makeButtons(...)
	local LABEL = "Play"
	local buttons = {}
	for slot = 1, 2 do
		buttons[slot] = function()
			return LABEL
		end
	end
	return buttons
end
local buttons = makeButtons()
print(buttons[1] == buttons[2], buttons[2]())

local binds = {}
for n = 1, 2 do
	local action = "Jump"
	binds[n] = function()
		return action
	end
end
print(binds[1] == binds[2], binds[1](), binds[2]())

local function scaler(...)
	local SCALE = 4
	return function(v)
		return v * SCALE
	end
end
print(scaler() == scaler(), scaler()(5))

local function bound(owner, ...)
	local TAG = "t"
	return function()
		return TAG .. owner
	end
end
print(bound("a") == bound("a"), bound("b")())
