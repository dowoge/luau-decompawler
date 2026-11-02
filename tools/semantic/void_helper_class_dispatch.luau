-- A void helper picking one action by kind through an if/elseif chain, copied twice into
-- the same caller: every copy takes the branch the call would.
local function entity(kind)
	local e = { kind = kind, trail = {} }
	function e:Is(k)
		return self.kind == k
	end
	function e:Teleport(where)
		self.trail[#self.trail + 1] = "tp:" .. where
		self.Hops = (self.Hops or 0) + 1
	end
	return e
end

local function moveTo(e, where)
	if e:Is("Prop") then
		e.Spot = where
	elseif e:Is("Player") then
		e:Teleport(where)
	elseif e:Is("Sound") then
		e.Origin = where
	end
end

local function relocate(e, where, owner)
	local before = #e.trail
	moveTo(e, where)
	moveTo(e, owner .. ":" .. where)
	e.Owner = owner
	return #e.trail - before, e.Hops or 0
end

local kinds = { "Sound", "Player", "Ghost", "Prop" }
local rows = {}
for i = #kinds, 1, -1 do
	local e = entity(kinds[i])
	local moved, hops = relocate(e, "z" .. i, "server")
	rows[#rows + 1] = table.concat({ e.kind, moved, hops, table.concat(e.trail, "|"), tostring(e.Spot), tostring(e.Origin), e.Owner }, " ")
end
print(table.concat(rows, "\n"))
