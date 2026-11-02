-- Locals named after builtin-like fields (`.Type`, `.Error`, `.Next`, `.Select`) or aliasing
-- a builtin must never hide a later read of the real global with the same spelling.
local Item = {}
function Item.describe(row)
	local kind = row.Type
	local problem = row.Error
	local following = row.Next
	local picked = row.Select
	local text = row.String
	local parts = {
		type(kind),
		kind,
		tostring(problem),
		tostring(next({ following })),
		tostring(select("#", picked, text)),
		string.reverse(text),
	}
	return table.concat(parts, "/")
end
print(Item.describe({ Type = "ice", Error = "oops", Next = 4, Select = "grab", String = "abc" }))

local type = type
local function kindOf(v)
	return type(v)
end
local countArgs = select
local function counts(...)
	return countArgs("#", ...), select(2, ...)
end
print(kindOf(2), kindOf("s"), kindOf(nil), counts("a", "b", "c"))

local function commit(self)
	local previous = self._VERY_UNSAFE_pendingWrite
	self._VERY_UNSAFE_pendingWrite = self._next
	local budget = self._MAX_retries
	return previous ~= self._next, budget
end
local function kinds(data)
	local list = {}
	for i, row in data.Rows do
		list[i] = row.Type
	end
	return list
end
local function label(row)
	local t = row.Type
	return t .. "=" .. type(t)
end
print(commit({ _VERY_UNSAFE_pendingWrite = 1, _next = 1, _MAX_retries = 3 }))
print(table.concat(kinds({ Rows = { { Type = "x" }, { Type = "y" } } }), "+"))
print(label({ Type = "fire" }), kindOf(true))
