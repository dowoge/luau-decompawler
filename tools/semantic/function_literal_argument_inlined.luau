-- A throttling helper inlined where callers hand it a function literal: the literal's
-- body runs inside the copy, still sees its captures, and every key keeps its own state.
local tick = 0
local stamps = {}

local function throttled(key, work)
	local last = stamps[key]
	if last ~= nil and tick - last < 3 then
		return "wait"
	end
	local result = work()
	if result ~= nil then
		stamps[key] = tick
	end
	return result
end

local function craft(item, ...)
	print("craft", item, select("#", ...))
	if item == "" then
		return nil
	end
	return item .. "!"
end

local recipes = {}
function recipes.torch()
	return throttled("torch", function()
		return craft("torch")
	end)
end
function recipes.custom(name)
	return throttled("custom", function()
		print("custom", name)
		return craft(name, name)
	end)
end
function recipes.logged(name)
	local outcome = throttled("logged", function()
		return craft(name, 1, 2, 3)
	end)
	print("logged", outcome)
	return outcome
end

local results = {}
for round = 1, 3 do
	tick = round * 2
	results[#results + 1] = recipes.torch() .. "/" .. tostring(recipes.custom(round == 2 and "" or "rope")) .. "/" .. tostring(recipes.logged("axe"))
end
print(table.concat(results, " "))

local bench = {}
bench.saw = function()
	return throttled("saw", function()
		return (craft("plank", "oak"))
	end)
end
bench.anvil = function()
	return throttled("anvil", function()
		return (craft("nail"))
	end)
end
local plan = { "saw", "anvil", "saw", "anvil" }
for i = #plan, 1, -1 do
	print(plan[i], bench[plan[i]]())
end
tick = 50
print(bench.saw(), bench.anvil())
