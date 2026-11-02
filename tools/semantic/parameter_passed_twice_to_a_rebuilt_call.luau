-- A copied-in helper passes its parameter twice to another helper whose call
-- is put back: the value is read once for each argument, so it is a local.
local function blend(a, b)
	return if string.len("ab") > b then 2 - a + b else -a % 5
end
local function twice(p)
	return blend(0.5, blend(p, p))
end
local results = {}
for i, v in {7, -3, 5, -3} do
	results[#results + 1] = twice(-i)
end
print(table.concat(results, " "))
