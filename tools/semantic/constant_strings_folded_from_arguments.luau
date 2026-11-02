-- Constant string arguments folded into a copy's keys, joined messages and interpolated
-- errors: each copy must read, write and report exactly the text the call would.
local function migrate(cfg, field, value)
	local from, to = `Old{field}`, `New{field}`
	local before = cfg[to]
	cfg[from] = cfg[from] or value
	cfg[to] = value
	return before ~= value, cfg[from]
end

local layers = { 0.2, 0.7 }
local function shade(night)
	local step = night and 0.25 or 0.05
	local total = 0
	for i = #layers, 1, -1 do
		layers[i] = math.min(layers[i] + step, 1)
		total += layers[i]
	end
	return total
end

local cfg = {}
local log = {}
for round = 1, 3 do
	local moved, first = migrate(cfg, "Volume", round == 3 and 4 or 8)
	local lit
	if round % 2 == 1 then
		moved = migrate(cfg, "Theme", "dark") or moved
		lit = shade(true)
	else
		moved = migrate(cfg, "Locale", "fr") or moved
		lit = shade(false)
	end
	log[#log + 1] = tostring(moved) .. ":" .. first .. ":" .. lit
end
print(table.concat(log, " "), cfg.NewVolume, cfg.OldTheme, cfg.NewLocale, cfg.OldVolume)

local function parseCount(what, text)
	local digits = text:match("^%s*(%-?%d+)%s*$")
	if not digits then
		if text:find("%S") == nil then
			return 1, nil
		end
		return nil, what .. " needs digits"
	end
	local n = tonumber(digits)
	if n < 0 then
		return nil, what .. " cannot be negative"
	end
	return n, nil
end

local function order(fruitText, crateText)
	local fruit, problem = parseCount("Fruit", fruitText)
	if problem then
		return problem
	end
	local crates
	crates, problem = parseCount("Crates", crateText)
	if problem then
		return problem
	end
	return fruit * crates
end
print(order("12", " 3 "), order("x", "1"), order("4", "-2"), order("", "5"))

local function want(index, fn, kinds, got)
	if not string.find(kinds, typeof(got), 1, true) then
		error(`#{index} to {fn}: wanted {kinds}, got {typeof(got)}`, 3)
	end
end

local Bank = {}
function Bank.deposit(account, amount, memo)
	want(1, "Bank.deposit", "table", account)
	want(2, "Bank.deposit", "number", amount)
	want(3, "Bank.deposit", "string/nil", memo)
	return amount * 2
end

local function try(...)
	local ok, err = pcall(...)
	if ok then
		return "ok " .. tostring(err)
	end
	return (tostring(err):gsub("^[^:]*:%d+: ", ""))
end
print(try(Bank.deposit, {}, 5), try(Bank.deposit, {}, "five"), try(Bank.deposit, 7, 1), try(Bank.deposit, {}, 1, true))
