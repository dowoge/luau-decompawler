-- Table constructors passed straight into inlined helpers that store into them or read
-- them after another call: entries, overwritten fields and order all survive.
local shelf = {}
local function stock(code, item)
	item.Code = code
	item.Qty = item.Qty or 1
	shelf[#shelf + 1] = item
	return #shelf
end

local first = stock("A1", { Qty = 4, Tags = { "red", "big" }, Size = { W = 2, H = 3 } })
local second = stock("B7", { Code = "old", Price = { 5, 9 } })
local third = stock("C3", {})
print(first, second, third, shelf[2].Code, shelf[2].Qty, shelf[3].Qty, shelf[1].Size.H)

local log = {}
local function labelOf(result)
	return type(result) == "table" and result.label or nil
end

local function announce(api, title, result)
	local label = labelOf(result)
	if not label then
		api.say(title)
		return
	end
	api.say(title .. " -> " .. label)
end

local function request(api, verb, body, title)
	local ok, result = api.send(verb, body)
	if ok then
		announce(api, title, result)
		return labelOf(result)
	end
	api.say("failed " .. tostring(result))
	return nil
end

local api = {
	send = function(verb, body)
		if verb == "nuke" then
			return false, "denied"
		end
		if verb == "ping" then
			return true, "pong"
		end
		return true, { label = verb .. ":" .. tostring(body.count or 0) }
	end,
	say = function(text)
		log[#log + 1] = text
	end,
}

local outcomes = {}
local jobs = {
	function()
		return request(api, "sync", {}, "Synced")
	end,
	function(n)
		return request(api, "buy", { count = n }, ("Bought %d"):format(n))
	end,
	function()
		return request(api, "ping", { count = 1 }, "Pinged")
	end,
	function()
		return request(api, "nuke", {}, "Nuked")
	end,
}
for i, act in ipairs(jobs) do
	outcomes[i] = tostring(act(i))
end
print(table.concat(outcomes, " "))
print(table.concat(log, " ; "))
