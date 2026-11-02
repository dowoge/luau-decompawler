-- Loop variables captured by closures are fresh each trip; a change made in a trip after
-- the capture shows in that trip's closure only, with `continue` and `break` in the way too.
local function multipliers(bases)
	local readers = {}
	for i, value in ipairs(bases) do
		value = value * i
		readers[i] = function()
			return value
		end
		if i % 2 == 0 then
			value = -value
		end
	end
	return readers
end
local m = multipliers({ 5, 8, 3 })
print(m[1](), m[2](), m[3]())

local function fillIn(template, words)
	for slot, word in ipairs(words) do
		if slot == 1 then
			word = string.upper(word)
		end
		template = string.gsub(template, "%%" .. slot, function()
			return word
		end)
	end
	return template
end
print(fillIn("%1 meets %2", { "kai", "rio" }))

local function ticketsUntil(list)
	local fns = {}
	for _, v in ipairs(list) do
		if v < 0 then
			continue
		end
		v = v + 1
		fns[#fns + 1] = function()
			return v
		end
		if v >= 10 then
			break
		end
	end
	local out = {}
	for i, f in ipairs(fns) do
		out[i] = f()
	end
	return table.concat(out, ",")
end
print(ticketsUntil({ 2, -1, 4, 9, 30 }), ticketsUntil({}))

local function firstBig(list)
	for _, v in ipairs(list) do
		if v > 10 then
			return function()
				return v * 2
			end
		end
	end
	return nil
end
local f = firstBig({ 3, 12, 40 })
print(f and f(), firstBig({ 1 }) == nil)

local byKey = {}
for key, amount in pairs({ Wood = 3 }) do
	if key == "Wood" then
		amount *= 10
	end
	byKey[#byKey + 1] = function()
		return key, amount
	end
	amount += 1
end
print(byKey[1]())
