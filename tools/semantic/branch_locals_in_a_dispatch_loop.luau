-- `local a, b, c, d` declared in each arm of an `if` inside a loop: the back
-- jump brings one arm's last values to another arm's declarations, and they
-- were once all one variable for the whole function (`a = nil` in each arm).
-- Each arm's are its own; closures made in an arm keep that arm's, also
-- when a call before the test used those registers for its arguments.

local saved = {}
local function run(code)
	local pc = 1
	while true do
		local op = code[pc]
		if op == 1 then
			local a, b, c, d
			a = code[pc + 1]
			b = a + 1
			saved[#saved + 1] = function()
				return a, b, c, d
			end
			print(a, b)
		elseif op == 2 then
			local a, b, c, d
			c = code[pc + 2]
			d = c * 2
			if c > 3 then
				b = d - 1
			else
				b = c + 1
			end
			print(c, d, b, a)
		elseif op == 3 then
			local a, b
			b = code[pc + 1]
			a = b .. "!"
			saved[#saved + 1] = function()
				return a, b
			end
			print(a)
		else
			return pc
		end
		pc += 3
	end
end
print(run({ 1, 5, 0, 2, 0, 7, 3, "x", 0, 1, 8, 0, 2, 0, 1, 9 }))
for _, f in saved do
	print(f())
end

local traced = {}
local function trace(code)
	local pc = 1
	while true do
		local op = code[pc]
		-- temporaries in the registers the arms declare in, before the test
		print("at", pc, code[pc + 1], code[pc + 2])
		if op == 1 then
			local a, b, c
			a = code[pc + 1]
			b = a * 10
			traced[#traced + 1] = function()
				return a, b, c
			end
		elseif op == 2 then
			local a, b
			b = code[pc + 2]
			a = b - 1
			traced[#traced + 1] = function()
				return b, a
			end
		else
			return pc
		end
		pc += 3
	end
end
print(trace({ 1, 4, 0, 2, 0, 6, 1, 7, 0, 9 }))
for _, f in traced do
	print(f())
end
