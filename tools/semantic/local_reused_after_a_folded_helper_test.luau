-- An `if` tests a copied-in helper that returns nothing; its body's local
-- lands in the register the test used and is read twice there. That is a
-- block with a local, not `x = nil or (...)`.
local function quiet(a, b)
end
local stock = {7, 4, 8, 2}
if not quiet(not 1, #stock) then
	local left = #stock
	left -= left
end
if not quiet(1, #stock) then
	local twice = #stock
	twice *= twice
end
print(#stock)
