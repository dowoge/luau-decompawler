-- Damage helpers are inlined at -O2 and rebuilt as calls: an argument that logs, or that
-- touches a captured local, must still run exactly where the inlined copy ran it.
local events = {}
local Probe = {}
function Probe.log(tag, v)
	events[#events + 1] = tag
	return v
end

local critMultiplier = 1.5
local function crit(base)
	return math.floor(base * critMultiplier + 0.5)
end
local function falloff(distance, base)
	return base * (1 - math.clamp((distance - 8) / 40, 0, 1) * 0.75)
end
local function wrapAngle(deg)
	return (deg % 360 + 360) % 360
end
local function turn(deg, quarter)
	return wrapAngle(deg + quarter * 90)
end
local function delayed(bonus)
	return Probe.log("roll", 3) * bonus
end
local function absorb(amount)
	amount = math.clamp(amount, 0, 50)
	if amount > 20 then
		return amount - 20
	end
	return -amount
end

print(crit(Probe.log("base", 21)), absorb(Probe.log("hit", 35)), absorb(Probe.log("graze", 5)))
critMultiplier = 2
print(falloff(Probe.log("dist", 30), Probe.log("dmg", 12)), crit(Probe.log("base2", 10)))
print(turn(Probe.log("deg", 350), 1), wrapAngle(Probe.log("raw", -30) - 720), delayed(Probe.log("bonus", 4)))

local armor = {}
local combo = 0
local function stamp(slot, amount)
	armor[slot] = amount
	events[#events + 1] = "stamp"
end
local function stampAfter(amount)
	events[#events + 1] = "after"
	armor.late = amount
end
local function swing(power)
	events[#events + 1] = "swing" .. power
	combo += 1
end
local function recover()
	combo += 10
end
stamp("chest", Probe.log("chestHit", 12))
stampAfter(Probe.log("legHit", 4))
for hit = 1, 3 do
	combo += hit
	swing(combo)
	recover()
end
print(combo, armor.chest, armor.late)
print(table.concat(events, " "))

local mana, charge = 5, 0
local Spell = {}
function Spell.cast()
	mana -= 2
	return 8
end
function Spell.meter()
	return charge
end
local function cost(n)
	print("mana", mana, "cost", n)
end
local function fill(shown)
	charge = 9
	print("fill", shown, charge)
end
cost(1)
print("mana", mana, "cost", Spell.cast())
fill(Spell.meter())
charge = 4
charge = 9
print("fill", Spell.meter(), charge)
