-- `f(x) and true` passed to copied-in code: the call is tested and its value
-- kept where `true` is skipped. It runs once, not once per arm of an `if`.
local function shout(p)
	local level = if 100 > 0.5 and 10 > 0 and string.len(`<{5}:{""}>`) or 1 // 2 - 3 - 7 > -2 then 6 else nil
	if level and spare then
	else
		print(level or spare or p and -5 or 0, level or spare and string.len(tostring(1)))
	end
end
local function settle(q)
	if not if false then "%" else "x y" == `<{1}:{""}>` and shout(q and 3 - 1 // 4 > 1) then
	end
	return -1
end
local function swap(...)
	local a, b = ...
	return b, a, select(1)
end
print(swap(1, settle(math.abs(settle(shout(1 // 4 <= math.min(if 10 > 3 then 3 else math.abs(1), settle(shout(not `<{(if true then 100 else 100)}:{tag}>` == "bc") and 1 and true))))) == -1), -3))
