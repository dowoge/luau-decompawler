-- Helpers stored into module fields while every direct call of them was inlined: the
-- fields still hold working functions, and the inlined copies give the same answers.
local Units = {}
local allowed = { metric = true, imperial = true }

local function toMeters(value)
	local n = tonumber(value)
	if n == nil then
		return 0
	end
	return if n < 0 then 0 else n * 0.3048
end

local function system(name)
	return if allowed[name] then name else "metric"
end

local function banner(s)
	return "== " .. s .. " =="
end

local function copyList(list, deep)
	if not deep then
		return table.clone(list)
	end
	local out = table.create(#list)
	for i, v in ipairs(list) do
		out[i] = type(v) == "table" and table.clone(v) or v
	end
	return out
end

Units.toMeters = toMeters
Units.banner = banner
Units.copyList = copyList

function Units.parse(cfg)
	local parsed = {}
	parsed.height = toMeters(cfg.height)
	parsed.width = toMeters(cfg.width)
	parsed.system = system(cfg.system)
	return parsed
end

function Units.flagged(list)
	local c = copyList(list)
	c.flagged = true
	return c
end

local facade = { system = system, parse = Units.parse }
local p = Units.parse({ height = "10", width = -4, system = "imperial" })
print(p.height, p.width, p.system, facade.system("lunar"), facade.system("metric"))
print(Units.toMeters("x"), Units.banner("ok"), Units.flagged({ 1 }).flagged, #Units.copyList({ 5, { 6 } }, true))
print(facade.parse({}).system, Units.toMeters(100))
