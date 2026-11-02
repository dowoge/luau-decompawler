local handler = nil
local function run(x)
	if x then
		return handler(x)
	end
	return 0
end
print(pcall(run, 1), run(false))
