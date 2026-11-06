-- A function so long that its outer jumps do not fit in 16 bits: the
-- compiler puts a JUMPX trampoline before each (`JUMP +1; JUMPX far;
-- OP -2`). Read as the jump it stands for, the nesting comes back; read
-- as written, every far test jumps backward into a goto.

local stats = { n = 0 }
local function t(s)
	if s == "stop1" then
		while true do
		end
	end
	if s == "stop2" then
		while true do
		end
	end
	if s == "stop3" then
		while true do
		end
	end
	if s == "stop4" then
		while true do
		end
	end
	if s == "stop5" then
		while true do
		end
	end
	if s == "stop6" then
		while true do
		end
	end
	if s == "stop7" then
		while true do
		end
	end
	if s == "stop8" then
		while true do
		end
	end
	stats.n += 1
	return s ~= stats
end

local function b(s)
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
if s and t(s) and t(s) and t(s) and t(s) and t(s) then
return true
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
end
stats.n += 100
return false
end

print(b("x"), stats.n)
stats.n = 0
print(b(false), stats.n)
