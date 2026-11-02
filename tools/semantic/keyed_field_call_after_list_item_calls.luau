-- A keyed field's value is a call put back for copied-in code that comes out
-- as a local of its own; the constructor's list items made their calls
-- first, so the table goes out before it.
local function stamp(v)
	print(v, math.max(1))
	return string.len(1)
end
local row = {stamp(1), tostring(1), k = stamp(absent)}
print(#row, row.k)
local function scaled(n)
	print(n, n)
	return 2 - n
end
local level = string.len(4)
local mixed = {math.max(level, scaled(spare and 1 or level - 0.5)), tostring(1), k = scaled(level) * -1}
print(#mixed, mixed.k)
