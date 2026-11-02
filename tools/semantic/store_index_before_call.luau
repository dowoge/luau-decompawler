local list = {}
local function add(v)
	list[#list + 1] = v
	return function(x)
		return x * 2
	end
end
local function run()
	local f = add(1)
	list[#list + 1] = f(5)
	return #list, list[1], list[2]
end
print(run())
