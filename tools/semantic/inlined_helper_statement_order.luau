-- Helpers with leading statements are inlined where their value feeds a call,
-- store or condition; what the source evaluates first must still run first.
local Source = {}
function Source.fetch(v)
	print("fetch", v)
	return v
end

local function stars(text)
	local width = #tostring(text)
	if width > 6 then
		return "*"
	end
	return string.rep("*", width) .. "|"
end

local Bus = {}
function Bus:emit(topic, payload)
	print("emit", topic, payload)
end

local labels = {}
Bus:emit("title", stars(Source.fetch("abc")))
labels.short = stars(Source.fetch(12345678))
print(Source.fetch("left"), stars(Source.fetch(42)))
if #stars(Source.fetch("go")) > 2 then
	print("wide", labels.short)
end

function measure()
	return 3
end
local function quiet(n)
	print("quiet", n)
end
local function loud(n)
	print("LOUD", n)
end
local speaker = quiet
local function prepare()
	speaker = loud
	return measure()
end
local function announce()
	speaker = loud
	speaker(measure())
end
announce()
speaker = quiet
print(prepare())
speaker = quiet
speaker(prepare())

local function animate(model, from, to)
	local t = 1
	local light = model
	local function blend(k)
		local w = math.min(1, t / 4)
		return { r = from.r + (to.r - from.r) * w * k, g = from.g }
	end
	light.Color = blend(1)
	local function detach()
		light = nil
	end
	return function(dt)
		t += dt
		if t > 6 then
			detach()
			return "detached"
		end
		light.Color = blend(2)
		return light.Color.r, light.Color.g
	end
end
local lamp = {}
local step = animate(lamp, { r = 0, g = 5 }, { r = 8, g = 9 })
print(lamp.Color.r, lamp.Color.g)
print(step(1))
print(step(10))
