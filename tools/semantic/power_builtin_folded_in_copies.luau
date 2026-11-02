-- A truncating helper whose builtin calls fold away where it is copied with constant
-- digits, while calls that stay real compute them at run time: results must agree.
local function truncate(x, decimals)
	local unit = math.pow(10, decimals)
	return math.floor(x * unit) / unit
end

local formats = {
	money = function(v)
		return truncate(v, 2)
	end,
	permille = function(v)
		return truncate(v / 1000, 3) * 1000
	end,
	whole = function(v)
		return truncate(v, 0)
	end,
}

local function display(kind, v)
	local f = formats[kind]
	if f then
		v = f(v)
	else
		v = truncate(v, 4)
	end
	return v
end

local function checked(x, decimals)
	local unit = math.pow(10, decimals)
	if type(x) ~= "number" and type(x) ~= "string" then
		print("refuse", x, 1) print("refuse", x, 2) print("refuse", x, 3)
		print("refuse", x, 4) print("refuse", x, 5) print("refuse", x, 6)
		error("cannot truncate", 0)
	end
	return math.floor(x * unit) / unit
end

print(display("money", 12.3456), display("permille", 987.6543), display("whole", -2.5), display("other", 3.14159265))
local raw = tonumber("5.98765")
local shown = checked("5.98765", 2)
print(shown, checked(raw, 3), truncate(raw, 1), (pcall(checked, false, 1)))
