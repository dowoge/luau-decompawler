-- A numeric loop followed by `continue` exits straight to the next pass of
-- the loop around it: the compiler sends its FORNPREP past the continue, to
-- where a later loop on the same registers ends too.
local function run(list)
	for i, x in ipairs(list) do
		if x > 0 then
			for j = 1, x do
				print("first", j, x)
			end
			continue
		end
		for k = i, 1, -1 do
			if x + k < -4 then
				break
			end
			print("second", k, x)
		end
	end
end
run({ -1, -5, 2, -2, 3 })

-- and one whose body is only `break`: its FORNLOOP jumps past the break
local function h(n, t)
	for i = 1, n do
		break
	end
	for i = 1, n do
		t[i] = i * 2
	end
	return #t
end
print(h(3, {}), h(0, {}))
