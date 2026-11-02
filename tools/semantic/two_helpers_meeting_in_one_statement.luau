-- Copies of two helpers in one statement, a nested helper holding a copy of another,
-- and a wrapper that only calls one: each value and effect comes out exactly once.
local wind = { t = 3 }
function wind:gust(lo, hi)
	self.t = (self.t * 41 + 7) % 89
	return lo + (hi - lo) * self.t / 89
end

local function percent(v)
	return math.clamp(math.floor(v * 100 + 0.5), 0, 100)
end

local function forecast(base, jitterAmount)
	local s = jitterAmount or 0.15
	local function sample(v)
		return percent(v + wind:gust(-s, s))
	end
	local readings = { sample(base.rain), sample(base.cloud), sample(base.fog) }
	return #readings, table.concat(readings, "/")
end

local function heatIndex(t, h)
	return t + 0.5 * h - 10
end

local function chill(t, w)
	return math.round((t - w * 0.7) * 10) / 10
end

local function summarize(t, h, w)
	return string.format("%.1f %.1f", heatIndex(t, h), chill(t, w))
end

local alerts = { active = nil, archive = {} }
local function retire()
	local a = alerts.active
	if a then
		alerts.active = nil
		alerts.archive[#alerts.archive + 1] = a.code
	end
end
local function silence()
	retire()
end
local function raise(code)
	local replaced = alerts.active and alerts.active.code or "-"
	silence()
	alerts.active = { code = code, replaced = replaced }
	return replaced
end

local station = { forecast = forecast, summarize = summarize }
print(station.forecast({ rain = 0.3, cloud = 0.8, fog = 0.05 }))
print(station.forecast({ rain = 1.2, cloud = -0.2, fog = 0.5 }, 0.4))
print(station.summarize(30, 60, 10), station.summarize(5, 20, 25))
alerts.active = { code = "W1" }
print(raise("W2"), raise("W3"), alerts.active.replaced)
retire()
print(table.concat(alerts.archive, ","), alerts.active)
