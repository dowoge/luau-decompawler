--!native
-- The module-wide native directive leads the file; whatever the output keeps of
-- it, the module must compute the same.
local function triangle(n)
	local acc = 0
	for i = 1, n do
		acc += i * (i + 1) / 2
	end
	return acc
end

print(triangle(5), triangle(40))
