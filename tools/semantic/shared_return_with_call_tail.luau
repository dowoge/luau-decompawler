-- A shared return that holds a method call after nested branches, and a returned `or`
-- headed by a call: each call runs once, after the branches, with the same fallback.
local Order = {}
Order.__index = Order
function Order:Total()
	if not self.cached then
		local sum = 0
		for _, line in ipairs(self.lines) do
			sum += line
		end
		if sum > 100 then
			self.cached = sum * 0.9
		elseif sum > 0 then
			print(self.id, "small order")
			self.cached = sum
		else
			print(self.id, "empty")
			self.cached = 0
		end
	end
	return self.coupon:Apply(self) or self.cached
end

local function leadingCount(text)
	return tonumber(text:match("^(%d+)")) or 0
end

local coupon = {
	Apply = function(_, order)
		if order.vip then
			return order.cached / 2
		end
		return nil
	end,
}
local orders = {
	setmetatable({ id = "o1", lines = { 60, 70 }, coupon = coupon }, Order),
	setmetatable({ id = "o2", lines = { 5 }, coupon = coupon, vip = true }, Order),
	setmetatable({ id = "o3", lines = {}, coupon = coupon }, Order),
}
for _, o in ipairs(orders) do
	print(o.id, o:Total(), o:Total())
end
print(leadingCount("12 apples"), leadingCount("apples"), leadingCount("7"))
