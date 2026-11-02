-- Helpers returning several values through guards, branches, a trailing literal and a
-- passed-in parameter: every copy hands each target its own value.
local function parsePair(text)
	local key, raw = string.match(text, "^%s*(%w+)%s*=%s*(%S*)%s*$")
	if key == nil then
		return nil, "malformed"
	end
	if raw == "" then
		return key, nil
	end
	local n = tonumber(raw)
	if n == nil then
		return nil, key .. " not numeric"
	end
	return key .. ":" .. math.floor(n), nil
end

local function settingLine(text)
	local entry, problem = parsePair(text)
	if problem then
		return "!" .. problem
	end
	return entry or "?"
end

local function settingKey(text)
	local entry, problem = parsePair(text)
	if not problem then
		return entry
	end
	return #problem
end

for _, text in ipairs({ "speed = 4.7", "mode =", "x = y", "???", " jump=12 " }) do
	print(settingLine(text), settingKey(text))
end

local function dims(panel)
	local w = panel.w
	local h = panel.h
	panel.area = w * h
	panel.w = w + 1
	return w, h, 0.5
end

local p1, p2 = { w = 3, h = 4 }, { w = 10, h = 2 }
local wA, hA, padA = dims(p1)
local wB, hB, padB = dims(p2)
local laps = 0
repeat
	laps += 1
	print(wA + padA * laps, hA, wB - padB, hB * laps, padA + padB)
until laps == 2
print(p1.area, p2.area, p1.w, p2.w)

local function tagAll(items)
	local tags = {}
	for i, item in ipairs(items) do
		tags[i] = "#" .. item.id
	end
	return tags, items
end

local function scaleCount(items, factor)
	factor *= #items
	return #items, factor
end

local function makeItems(n)
	local out = table.create(n)
	for i = n, 1, -1 do
		out[i] = { id = i * 11 }
	end
	return out
end

local function check(n)
	local tags, same = tagAll(makeItems(n))
	print(#tags, tags[1], #same, same[n].id)
	local count, factor = scaleCount(same, 2)
	print(count, factor)
end
check(3)
