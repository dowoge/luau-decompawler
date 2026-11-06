-- Loops whose exits hang on globals nobody assigns (`_`, `__` read nil), a
-- repeat that only ever runs once, and a `continue` that can never be taken:
-- each test must stay where it is, as must the writes between them.

local ticks = 0
function wait()
	ticks += 1
	return ticks < 3
end

print(1)
local fn = function()
	print("This might not appear")
	while wait() do
	end
end

local fn2 = function()
	return "This will definitely appear"
end

local callback = fn2
while true do
	if callback ~= fn2 then break end
	callback = fn
	repeat
		if not _ then break end
	until true
	if not callback then continue end
end

callback()
print(2, ticks)
local n2 = 1
while true do
	if not (n2 <= 1) then break end
	n2 = 12345
	repeat
		if not __ then break end
	until true
	if not n2 then continue end
end

print("might be 1, might be", n2)
if n2 > 5 then
	print("running 5 < a")
end

local n = n2 / 5
print("Maybe 1/5, maybe", n)
if n % 1 == 0 then
	print("b%1 == 0")
end

-- the same shapes with the globals set, so the other ways out run too
_, __ = true, true
local passes = 0
local cb = fn2
while true do
	passes += 1
	if cb ~= fn2 or passes > 4 then break end
	repeat
		if not _ then break end
		cb = if passes > 2 then fn else fn2
	until true
	if not cb then continue end
end
print(passes, cb == fn)

local m = 1
while true do
	if not (m <= 100) then break end
	m *= 3
	repeat
		if not __ then break end
		m += 1
	until true
	if not m then continue end
end
print(m)
