-- Helpers that return two values through branches, or fuse two returns into `c and k or x`,
-- are inlined at -O2; rebuilt as calls, every site must keep its own argument and results.
local function parseCommand(line: string)
	local space = string.find(line, " ")
	if space then
		return string.sub(line, 1, space - 1), string.sub(line, space + 1)
	end
	return line, ""
end
local function sanitize(text: string)
	local clean = text:gsub("\r", "")
	local hasLt = clean:find("<")
	local hasAmp = clean:find("&")
	if hasLt and hasAmp then
		return clean:gsub("<", "&lt;"), "both"
	end
	if hasLt then
		return clean, "angle"
	end
	return clean, "plain"
end
local function run(lines)
	local out = {}
	for _, line in lines do
		local verb, rest = parseCommand(line)
		local safe, kind = sanitize(rest)
		out[#out + 1] = verb .. "(" .. safe .. ")" .. kind
	end
	table.sort(out, function(a, b)
		local va, ra = parseCommand(a)
		local vb, rb = parseCommand(b)
		if va ~= vb then
			return va < vb
		end
		return ra < rb
	end)
	return table.concat(out, " ; ")
end
print(run({ "kick bob", "say a<b&c", "ban", "say hi<" }))

local function speedOf(part)
	if part.Mass <= 1e-4 then
		return "weightless"
	end
	return part.Force / part.Mass
end
local function compare(a, b, c)
	local TINY = 1e-4
	local first = speedOf(a)
	local second = speedOf(b)
	if c.Mass > TINY then
		local third = speedOf(c)
		if c.Mass - TINY <= 2 then
			return third, second
		end
	end
	return first, second
end
print(compare({ Mass = 0, Force = 3 }, { Mass = 2, Force = 8 }, { Mass = 1, Force = 9 }))
print(compare({ Mass = 4, Force = 8 }, { Mass = 0, Force = 1 }, { Mass = 5, Force = 1 }))

local Probe = {}
function Probe.echo(v)
	print("echo", v)
	return v
end
local function volume(input)
	local n = tonumber(input)
	if not n then
		return 50
	end
	return math.clamp(math.floor(n), 0, 100)
end
local v1 = volume(Probe.echo("72.6"))
local v2 = volume(Probe.echo("loud"))
local v3 = volume(v1 * 3)
if v3 ~= v1 then
	print("clamped", v3)
end
print(v1, v2, v3)
