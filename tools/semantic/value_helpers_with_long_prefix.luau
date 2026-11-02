-- Value helpers whose copies run several statements, branches included, before their
-- value: every copy leaves the same value and the same effects as the call would.
local created = {}
local styleCache = { title = "Bold" }

local function label(parent, text)
	local l = { Kind = "Label" }
	l.Text = text
	l.Size = #text * 7
	l.Visible = true
	l.ZIndex = 2
	l.Parent = parent
	created[#created + 1] = text
	return l
end

local function container(screen)
	local frame = screen.Frame
	if frame == nil then
		frame = { Kind = "Frame", Children = 0 }
		screen.Frame = frame
	end
	frame.Children += 1
	return frame
end

local function styleFor(theme, role)
	local chosen = theme.custom and theme.custom[role] and ("custom:" .. theme.custom[role])
	if chosen then
		return chosen
	end
	if styleCache[role] ~= nil then
		return styleCache[role]
	end
	return "Plain" .. role
end

local function caption(item)
	if item == nil then
		return nil
	end
	return item.title:upper() .. "?"
end

local seats = {}
local function seatFor(guest)
	local ticket = guest and guest.ticket
	if ticket then
		local reserved = seats[guest]
		if reserved ~= nil and reserved.ticket == ticket then
			return reserved
		end
		local fresh = ticket.seat
		seats[guest] = fresh
		return fresh
	end
	return nil
end

local UI = {}
function UI.compose(screen)
	local heading = label(screen, "Title")
	local body = label({}, "Body")
	body.Parent = container(screen)
	local again = container(screen)
	return heading.Parent == screen, body.Parent.Kind, again.Children, table.concat(created, ",")
end
function UI.styles(theme)
	local list = {}
	for _, role in ipairs({ "title", "body", "footer" }) do
		local s = styleFor(theme, role)
		list[#list + 1] = s
	end
	return table.concat(list, " ")
end
function UI.captions(a, b)
	local top = caption(a)
	local second = caption(b)
	return tostring(top) .. "|" .. tostring(second)
end
function UI.seated(guest)
	local s = seatFor(guest)
	return s ~= nil and s.row or -1
end

print(UI.compose({}))
print(UI.styles({ custom = { body = "Mono" } }), UI.styles({}))
print(UI.captions(nil, { title = "menu" }), UI.captions({ title = "x" }, nil))
local ticket = {}
ticket.seat = { ticket = ticket, row = 12 }
local guest = { ticket = ticket }
print(UI.seated(guest), UI.seated(guest), UI.seated(nil), UI.seated({ ticket = {} }))
