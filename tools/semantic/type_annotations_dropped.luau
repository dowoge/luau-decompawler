-- Typed parameters and returns (optional, function, table, vector and buffer
-- types) compile to the same code; the round trip must keep the behaviour.
local Shop = {}
function Shop.price(base: number, discount: number?, currency: string): string
	local total = base * (1 - (discount or 0))
	return currency .. tostring(total)
end

function Shop:open(enabled: boolean, onOpen: (string) -> (), stock: { string }, at: vector, data: buffer)
	if enabled then
		onOpen(stock[1])
	end
	return self, at, data
end

local function sum(get: () -> number, times: number): number
	local acc = 0
	for _ = 1, times do
		acc += get()
	end
	return acc
end

print(Shop.price(100, 0.25, "$"), Shop.price(40, nil, "G"))
print(sum(function() return 4 end, 5))
local s, at, data = Shop:open(true, function(item) print("opened", item) end, { "apple" }, vector.create(1, 2, 3), buffer.create(2))
print(s == Shop, at.y, buffer.len(data))
return Shop
