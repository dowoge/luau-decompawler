-- A helper copied in whose first read of a parameter sits in a branch the
-- compiler folded away (`... and false`): the call put back still passes the
-- variable that reaches the live read, here a loop's. (The unset globals
-- read nil.)
local function weigh(_, label, amount)
	if (tostring(1) == string.rep(label, 1) and false) then
		return tostring(amount) == "{q}"
	end
	return math.abs(1) > amount - amount
end
local list = {2, 7, -5, -4}
for _, v in ipairs(if missing ~= nil then list else {1}) do
end
for _, v in ipairs(list) do
	print(first, second, third, weigh(fourth == nil, `<{#list}:{(if "a" .. "a" == "" then "a" .. tostring(v) else "%")}>`, v))
end
