-- `banner`'s body reads `hp` before its parameter. A copy of that body whose
-- argument slot calls `strike` (which lowers `hp`) must not become `banner(strike())`.
local hp = 100
local Combat = {}
function Combat.strike()
	hp -= 30
	return 7
end
local function banner(dmg)
	return ("<hp %d, hit %d>"):format(hp, dmg)
end
print(banner(1))
print(("<hp %d, hit %d>"):format(hp, Combat.strike()))
print(hp)
