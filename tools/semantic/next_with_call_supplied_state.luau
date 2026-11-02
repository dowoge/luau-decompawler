-- `for k, v in next, folder:GetChildren() do`: the call fills the state slot (and the control
-- slot when it gives two values); the loop must keep that shape, varargs included.
local Folder = {}
Folder.__index = Folder
function Folder.new(names)
	return setmetatable({ names = names }, Folder)
end
function Folder:GetChildren()
	return self.names
end
function Folder:Pair()
	return self.names, nil
end

local function listAll(...)
	for i, name in next, ... do
		print("vararg", i, name)
	end
end
local function walk(folder)
	for i, name in next, folder:Pair() do
		print("pair", i, name)
	end
	for i, name in next, folder:GetChildren() do
		print("children", i, name)
	end
end
local function hasChild(folder, wanted)
	local found = false
	for _, name in next, folder:GetChildren() do
		if name == wanted then
			found = true
			break
		end
	end
	return found
end
local function crossCount(folder)
	local n = 0
	for _, a in next, folder.GetChildren(folder) do
		for _, b in next, folder:GetChildren() do
			if a < b then
				n += 1
			end
		end
	end
	return n
end
local function custom(gen, folder)
	for i, name in gen, folder:GetChildren() do
		print("gen", i, name)
	end
end
listAll({ "Spawn", "Lobby" })
local folder = Folder.new({ "Map", "Lights", "Props" })
walk(folder)
print(hasChild(folder, "Props"), hasChild(folder, "Sky"), crossCount(folder))
custom(next, folder)
