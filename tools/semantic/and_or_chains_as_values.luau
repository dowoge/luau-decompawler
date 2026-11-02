-- `x > hi and hi or x < lo and lo or x` and `ok and v or default` are values;
-- inlined copies landing in locals, fields, loops and returns must agree.
local function bound(lo, hi, x)
	return (x > hi and hi or x < lo and lo or x)
end

local function regen(hp, maxHp, ticks)
	for _ = 1, ticks do
		hp = bound(0, maxHp, hp + 7)
	end
	return hp
end

local settings = {}
local function setVolume(input)
	settings.Volume = bound(0, 10, input)
	settings.Muted = bound(0, 10, input) == 0
	return settings.Volume
end

local function aimAt(spread, target)
	local limit = spread * 0.5
	local yaw = bound(-limit, limit, target.yaw)
	local pitch = bound(-limit, limit, target.pitch) or 0
	return yaw, pitch
end

print(regen(5, 30, 2), regen(25, 30, 3), regen(-50, 30, 1))
print(setVolume(14), settings.Muted, setVolume(-3), settings.Muted, setVolume(4))
print(aimAt(2, { yaw = 3, pitch = -0.25 }))

local Saved = { Theme = "dark", Scale = 2 }
local function read(store, key, default)
	if key == nil then
		return default
	end
	local ok, value = pcall(function()
		return store[key]
	end)
	return ok and value or default
end
local function profile(keys)
	local p = {}
	p.Theme = read(Saved, keys.theme, "light")
	p.Scale = read(Saved, keys.scale, 1)
	p.Extra = {
		Mode = read(Saved, keys.mode, nil),
		Speed = keys.speed or 16,
	}
	return p
end
for _, keys in { { theme = "Theme", mode = "Scale" }, { scale = "Scale", speed = 20 }, { theme = "Missing" } } do
	local p = profile(keys)
	print(p.Theme, p.Scale, p.Extra.Mode, p.Extra.Speed)
end
print(read(nil, "x", "fallback"), read(Saved, "Scale", 0))
