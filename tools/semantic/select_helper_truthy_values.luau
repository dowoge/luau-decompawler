-- Value helpers choosing by early returns or by and/or, copied into arithmetic and
-- tables: constant bounds and truthy arms must pick exactly what each call picks.
local turbo = false
local gearbox = { ratio = 4 }
function gearbox:Ratio()
	return self.ratio
end

local function gear()
	if turbo then
		return gearbox:Ratio()
	end
	return 2
end

local speeds = {}
for _, rpm in ipairs({ 1200, 3600, 3600 }) do
	speeds[#speeds + 1] = rpm / gear() / 100
	turbo = not turbo
	gearbox.ratio += 1
end
print(table.concat(speeds, ", "))

local function limit(v, lo, hi)
	if v > hi then
		return hi
	elseif v < lo then
		return lo
	else
		return v
	end
end

local function readPercent(raw, fallback)
	local n = tonumber(raw)
	if n == nil then
		return fallback
	elseif n > 100 then
		return 100
	end
	return if n < 0 then 0 else n
end

local Audio = {}
function Audio.mix(level)
	return { dry = 1 - limit(level or 0.5, 0, 1), wet = limit(level or 0.5, 0, 1) * 8 }
end
function Audio.load(cfg)
	local channel = {}
	channel.Volume = readPercent(cfg.Volume, 80)
	channel.Balance = readPercent(cfg.Balance, 50)
	return channel
end

for _, level in ipairs({ -1, 0.25, 2 }) do
	local m = Audio.mix(level)
	print("mix", level, m.dry, m.wet)
end
local m = Audio.mix()
print("mix default", m.dry, m.wet)
local ch = Audio.load({ Volume = "250", Balance = "left" })
print(ch.Volume, ch.Balance, Audio.load({ Volume = "-9" }).Volume, Audio.load({ Balance = "33.5" }).Balance)

local noiseState = { n = 5 }
function noiseState:Next(lo, hi)
	self.n = (self.n * 75 + 74) % 65537
	return lo + (hi - lo) * self.n / 65537
end

local function jitter(mag)
	return mag and noiseState:Next(-mag, mag) or 0
end

local function sample(base, mag)
	local point = { x = base + jitter(mag), y = base - jitter(mag) * 2 }
	local loud = jitter(mag) > 0
	return point.x, point.y, loud
end
print(sample(10, 0.5))
print(sample(10, nil))
print(sample(10, false))
