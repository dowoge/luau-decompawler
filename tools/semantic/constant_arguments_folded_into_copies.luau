-- Helpers inlined with constant arguments that the compiler folds into the copy's
-- arithmetic or into one constant: every folded value prints bit for bit.
local lines = {}
local function put(...)
	local parts = table.pack(...)
	for i = 1, parts.n do
		local v = parts[i]
		parts[i] = type(v) == "number" and string.format("%.17g", v) or tostring(v)
	end
	lines[#lines + 1] = table.concat(parts, " ", 1, parts.n)
end

local function beats(n: number)
	return n * 60 / 140
end

local function decibels(x: number)
	return 20 * x - 6
end

local function voice(id)
	local v = { Id = id, Release = 0 }
	function v:Mark(what, amount)
		put(self.Id, what, amount)
	end
	return v
end

local function shape(v, attack)
	v.Attack = attack / 8
	v.Release = 1 - attack
	v:Mark("peak", attack * 12)
end

local function squash(v, k)
	v.Curve = k ^ 2
	v.Steps = math.floor(k * 9)
end

local function span(x, lo, hi, a, b)
	return a + (x - lo) * (b - a) / (hi - lo)
end

local function bar(n: number)
	return { length = beats(n) * 4, unit = "bar" }
end

local phrase = {}
local v1, v2, v3 = voice("v1"), voice("v2"), voice("v3")
shape(v1, 0.6)
shape(v2, 0.35)
squash(v3, 0.7)
squash(v1, 1.1)
phrase[1] = { v1.Attack, v1.Release, v2.Attack, v2.Release, v3.Curve, v3.Steps, v1.Curve, v1.Steps }
phrase[2] = { beats(3), beats(7), beats(-2), decibels(0.3), decibels(-1.5) }
phrase[3] = { span(5, 0, 8, 0.1, 0.9), span(-2, -4, 4, 100, 300), bar(0.5).length, bar(3).length }
for _, row in ipairs(phrase) do
	put(table.unpack(row))
end

local function sequencer(play, rest)
	local function step(n: number)
		return { at = beats(n), kind = "step" }
	end
	play(step(5), "a")
	rest(beats(5))
	rest(2.142857142857143)
	play(step(9), "b")
end

local function written(play, rest)
	rest(0.42857142857142855)
	play({ at = 0.42857142857142855, kind = "step" }, "c")
end

local function afterwards(k)
	return function()
		return beats(k), beats(14)
	end
end

local function play(info, tag)
	put("play", tag, info.at, info.kind)
end
local function rest(t)
	put("rest", t, t * 140)
end
sequencer(play, rest)
written(play, rest)
local v4 = voice("v4")
shape(v4, 0.35)
put("afterwards", afterwards(6)())
for _, line in ipairs(lines) do
	print(line)
end
