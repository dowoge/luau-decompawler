-- A plain copy of a captured local is its own variable: closure writes to the original
-- miss the copy, and writes to the copy (dead ones too) never reach the original.
local Camera = {}
function Camera.zoom(fov, sniper)
	if sniper then
		fov = fov / 4
	end
	local nest = function()
		return function(v)
			fov = v
		end
	end
	local before = fov
	nest()(90)
	return before, fov
end
function Camera.peek(fov, sniper)
	if sniper then
		fov = fov / 4
	end
	local read = function()
		return fov
	end
	local before = fov
	local first = read()
	fov = 120
	return before, first, read(), fov
end
function Camera.widen(fov, sniper)
	if sniper then
		fov = fov / 4
	end
	local before = fov
	local read = function()
		return fov
	end
	fov = fov + 10
	return before, read(), fov
end
print(Camera.zoom(80, true))
print(Camera.peek(80, true))
print(Camera.widen(80, false))

local function shake(rig, strong)
	local amp = rig.base
	if strong then
		amp = rig.strong
	end
	if rig.verbose then
		print("verbose")
	end
	local setAmp = function(v)
		amp = v
	end
	local before = amp
	setAmp(0.5)
	local setters = {}
	for i = 1, 2 do
		setters[i] = function(v)
			amp = v * i
		end
	end
	local mid = amp
	setters[2](3)
	return before, mid, amp
end
print(shake({ base = 1, strong = 4 }, true))

local brightness = 2
local readBrightness = function()
	return brightness
end
brightness = 7
local shown = brightness
print(shown)
shown += 1
print(readBrightness())

local function bake(layers)
	local base, rate = nil, nil
	if layers.default then
		base = { "plain" }
		rate = 30
	end
	local rows = {}
	for _, layer in ipairs(layers) do
		local use = base
		local fps = rate
		if layer.override then
			use = { layer.name }
			fps = layer.fps
		end
		rows[#rows + 1] = (use and use[1] or "-") .. "@" .. tostring(fps or 60)
	end
	local function now()
		return base and base[1], rate
	end
	return table.concat(rows, " "), now()
end
print(bake({ default = true, { name = "fog", override = true, fps = 12 }, { name = "rain" } }))
print(bake({ { name = "snow", override = true } }))

local function relay(first, second, rounds)
	local holder, spare = first, second
	local seen = {}
	for r = 1, rounds do
		for _ = 1, 2 do
			if spare ~= nil then
				spare = holder .. r
				spare = holder
			end
			seen[#seen + 1] = holder
		end
		holder = spare
	end
	local peek = function()
		return holder
	end
	return table.concat(seen, " "), peek(), spare
end
print(relay("x", "y", 2))
