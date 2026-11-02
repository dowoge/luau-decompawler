-- `bindAction` is inlined everywhere, so its literal is one closure constant
-- for the chunk: the first copy to run seeds it; copies capturing other handlers build new closures.
local function bindAction(handler)
	return function()
		return handler
	end
end

local function onJump()
	return "jump"
end
local jumpBind = bindAction(onJump)

local crouchBinds = {}
local trips = 0
while trips < 2 do
	trips += 1
	local function onCrouch()
		return "crouch"
	end
	crouchBinds[trips] = bindAction(onCrouch)
end
print(crouchBinds[1] == crouchBinds[2], crouchBinds[2]()())
print(bindAction(onJump) == jumpBind, jumpBind()())

local function hotbar(...)
	local keys = {}
	for slot = 1, 2 do
		local function onUse()
			return "use" .. 1
		end
		keys[slot] = bindAction(onUse)
	end
	return keys
end
local h1, h2 = hotbar(), hotbar()
print(h1[1] == h1[2], h1[1] == h2[1], h2[2]()())
