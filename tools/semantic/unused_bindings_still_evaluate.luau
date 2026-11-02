-- A binding nobody reads still evaluates its value: an `__index` runs and records the
-- key, at chunk level and inside a function alike. `xpcall` results read like `pcall` ones.
local requested = {}
Packages = setmetatable({}, {
	__index = function(_, key)
		table.insert(requested, key)
		return { Name = key }
	end,
})
local Signal = Packages.Signal
local Promise = Packages.Promise
local Janitor = Packages.Janitor

local function mount(ctx)
	local theme = ctx.Theme
	local spacing = ctx.Spacing
	return #requested
end
print(mount(Packages), table.concat(requested, ","), Signal.Name)

local function withdraw(amount)
	if amount > 50 then
		error("insufficient", 0)
	end
	return 50 - amount
end
local function explain(message)
	return "denied: " .. message
end
local ok, left = xpcall(withdraw, explain, 20)
local okAgain, why = xpcall(withdraw, explain, 80)
print(ok, left, okAgain, why)
