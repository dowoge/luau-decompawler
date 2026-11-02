-- Helpers built from an outer local plus a method call are inlined at -O2; rebuilt as calls
-- (a short helper may match the start of a longer one) their effects keep their order.
local journal = {}
local toast = {}
function toast:Show(text)
	table.insert(journal, "toast " .. tostring(text))
end
local function notify(text)
	if text then
		toast:Show(text)
	end
end

local function openShop(menu, member)
	local function purchase(item)
		menu:Refresh()
		if menu.Validate(item) ~= false then
			table.insert(journal, "paid " .. tostring(item))
		end
	end
	local function close()
		menu:Refresh()
	end
	notify(member and "member")
	notify(nil)
	notify(menu.Title)
	if member then
		purchase(nil)
	else
		close()
	end
	purchase("sword")
	return purchase, close
end

local menu = { Title = "Armory", refreshes = 0 }
function menu:Refresh()
	self.refreshes += 1
	table.insert(journal, "refresh " .. self.refreshes)
end
function menu.Validate(item)
	return item ~= "sword"
end
openShop(menu, true)
openShop(menu, false)
print(table.concat(journal, " | "))
