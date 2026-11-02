-- Long and/or chains in conditions, loops, stores and returns keep operand order
-- and short-circuiting however they are wrapped; `check` is global, never inlined.
local visited = {}
function check(label, value)
	table.insert(visited, label)
	return value
end

local Weapon = {}
function Weapon.canFire(gun, holder)
	if check("no gun", gun == nil) or check("jammed", gun.Jammed == true) or check("empty clip", gun.Ammo <= 0) or check("holstered", gun.Holstered) then
		return "blocked"
	elseif check("holder alive", holder.Health > 0) and check("not stunned", holder.Stunned ~= true) and check("aiming", holder.Aiming == true) then
		return "fire"
	end
	local fallback = check("first reload", gun.Ammo < 3) and check("second reload", "reloading now") and check("third reload", "still reloading") or check("idle fallback", "waiting for input")
	return fallback
end

local function cooldown(heat)
	local turns = 0
	while check("still hot", heat > 0) and check("turn budget", turns < 40) and check("not frozen", heat ~= 4) and check("always", true) do
		heat -= 2
		turns += 1
	end
	repeat
		heat += 3
	until check("warmed up", heat >= 9) or check("overflow guard tripped", heat > 900) or check("never true", heat == -777)
	return turns, heat
end

local status = {}
local function equipped(bag)
	status.equipped = bag ~= nil and bag.Hotbar ~= nil and bag.Hotbar.Slots ~= nil and #bag.Hotbar.Slots > 0 and bag.Hotbar.Locked ~= true
	return status.equipped
end

local gun = { Jammed = false, Ammo = 2, Holstered = false }
for _, holder in { { Health = 0 }, { Health = 9, Aiming = true }, { Health = 9, Aiming = false } } do
	print(Weapon.canFire(gun, holder), table.concat(visited, ","))
	table.clear(visited)
end
print(Weapon.canFire({ Jammed = true, Ammo = 9 }, {}), table.concat(visited, ","))
table.clear(visited)
print(cooldown(8), table.concat(visited, ","))
print(equipped(nil), equipped({ Hotbar = { Slots = { "axe" } } }), equipped({ Hotbar = {} }), equipped({ Hotbar = { Slots = {}, Locked = true } }))
