-- A template field stored by both arms of a branch (seen first or not, captured,
-- grown, one arm only), and a template filled by garbage-churning calls.
local function choose(n)
	print("choose", n)
	return n
end
local function fields(t)
	local out = {}
	for k, v in pairs(t) do
		out[#out + 1] = k .. "=" .. tostring(v)
	end
	return table.concat(out, ",")
end

local Builders = {}
function Builders.unseen(left)
	local cell = { Tile = 0, Owner = 0 }
	if left then
		cell.Tile = choose("grass")
	else
		cell.Tile = choose("sand")
	end
	return cell
end
function Builders.seen(left)
	local cell = { Tile = 0, Owner = 0 }
	print("tile was", cell.Tile)
	if left then
		cell.Tile = choose("grass")
	else
		cell.Tile = choose("sand")
	end
	return cell
end
function Builders.captured(left)
	local cell = { Tile = 0, Owner = 0 }
	local function tile()
		return cell.Tile
	end
	print("captured", tile())
	if left then
		cell.Tile = choose("grass")
	else
		cell.Tile = choose("sand")
	end
	print("now", tile())
	return cell
end
function Builders.grown(left)
	local cell = { Tile = 0 }
	cell.Height = choose(3)
	if left then
		cell.Tile = choose("grass")
	else
		cell.Tile = choose("sand")
	end
	return cell
end
function Builders.oneArm(left)
	local cell = { Tile = 0, Owner = 0 }
	if left then
		cell.Tile = choose("grass")
	end
	return cell
end

for _, left in { true, false } do
	for _, name in { "unseen", "seen", "captured", "grown", "oneArm" } do
		print(name, fields(Builders[name](left)))
	end
end

local junk
loadModule = function(v)
	for i = 1, 12000 do
		junk = { i, tostring(i) }
	end
	return v
end
Modules = { Core = 1, Input = 2, Render = 3, Audio = 4, Net = 5, Save = 6, Ui = 7, Chat = 8 }
local loaded = {
	Core = loadModule(Modules.Core),
	Input = loadModule(Modules.Input),
	Render = loadModule(Modules.Render),
	Audio = loadModule(Modules.Audio),
	Net = loadModule(Modules.Net),
	Save = loadModule(Modules.Save),
	Ui = loadModule(Modules.Ui),
	Chat = loadModule(Modules.Chat),
}
print(fields(loaded))
