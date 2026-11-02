-- Helpers choosing between two computed values on a flag: a variable flag keeps both
-- arms, a constant or missing one keeps a single arm, and each copy gives the call's value.
local function price(base, member)
	local total
	if member then
		total = (base * 0.75 + 1) * 4
	else
		total = (base + 6) * 3
	end
	return total
end

local quotes = {}
for _, base in ipairs({ 4, 10 }) do
	for _, member in ipairs({ true, false }) do
		quotes[#quotes + 1] = price(base, member) .. "/" .. price(base, false)
	end
end
print(table.concat(quotes, " "))
local Shop = { price = price }
print(Shop.price(8, "yes"), Shop.price(8))

local toast
local function paint(r, g)
	return "rgb(" .. r .. "," .. g .. ")"
end
local function notify(text, urgent)
	if toast == nil then
		return "skipped"
	end
	toast.Text = text
	toast.Color = urgent and paint(255, 30) or paint(20, 200)
	return toast.Color
end

local function onScore(points)
	if points < 0 then
		return notify("penalty", true)
	end
	return notify("bonus")
end

local seen = { onScore(5) }
toast = {}
seen[#seen + 1] = onScore(-3)
seen[#seen + 1] = toast.Text
seen[#seen + 1] = onScore(12)
seen[#seen + 1] = toast.Text
print(table.concat(seen, " | "))
