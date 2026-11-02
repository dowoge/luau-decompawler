-- Inlined lookups and setters given constant false flags: a false flag stays false where
-- the helper passes it on, and every looked-up child or handle is the right one.
local warnedAbout = {}
local function warnFirst(key, ...)
	if not warnedAbout[key] then
		warnedAbout[key] = true
		print("notice", ...)
	end
end

local function fetchChild(parent, name)
	local found = parent:Find(name)
	if found == nil then
		warnFirst(name, "missing", name, "under", parent.Name)
	end
	return found
end

local function newNode(name)
	local kids = {}
	return {
		Name = name,
		Kids = kids,
		Find = function(self, n)
			return kids[n]
		end,
	}
end

local function setBorder(node, thick)
	local border = node:Find("Border")
	if border == nil then
		border = newNode("Border")
		node.Kids.Border = border
	end
	border.Size = thick and 3 or 1
	return border.Size
end

local function settingsButton(screen)
	local panel = fetchChild(screen, "Panel")
	local button = panel and fetchChild(panel, "Button")
	return button and button.Name or "none"
end

local screen = newNode("Screen")
print(settingsButton(screen))
screen.Kids.Panel = newNode("Panel")
print(settingsButton(screen), settingsButton(screen))
screen.Kids.Panel.Kids.Button = newNode("Button")
print(settingsButton(screen))
local a, b = newNode("A"), newNode("B")
print(setBorder(a, false), setBorder(b, true), setBorder(a, true), setBorder(b, false))

local out = {}
local function say(...)
	local parts = {}
	for i = 1, select("#", ...) do
		parts[i] = tostring((select(i, ...)))
	end
	out[#out + 1] = table.concat(parts, " ")
end

local function highlight(p, extra)
	say("shine", p.Name)
	if extra then
		p.Extra = (p.Extra or 0) + 1
	end
	p.Shine = 1
end

local function snapshot(t, deep)
	if not deep then
		return table.clone(t)
	end
	local dup = {}
	for k, v in pairs(t) do
		dup[k] = type(v) == "table" and snapshot(v, true) or v
	end
	return dup
end

local function setLimit(t, capped)
	t.limit = capped and 100
	say("limit", t.limit)
end

local function setFaded(p, faded)
	p.Alpha = faded and 0.3 or 1
	say("alpha", p.Alpha)
end

local function setMode(p, night)
	if night then
		p.Mode = "night"
	else
		p.Mode = "day"
	end
	say("mode", p.Mode)
end

local function flagUrgent(urgent, p)
	p.Visits = (p.Visits or 0) + 1
	if urgent then
		p.Urgent = "now"
	end
	say("urgent", p.Visits, p.Urgent)
end

local function apply(parts)
	local src = { n = 1, inner = { m = 2 } }
	local full = snapshot(src, true)
	setMode(parts.c, false)
	flagUrgent(false, parts.c)
	highlight(parts.b, true)
	setFaded(parts.c, false)
	local flat = snapshot(src, false)
	highlight(parts.a, false)
	setLimit(parts.c, false)
	return flat.inner == src.inner, full.inner == src.inner, full.inner.m
end
local parts = { a = { Name = "a" }, b = { Name = "b" }, c = { Name = "c" } }
local sameInner, deepInner, m = apply(parts)
print(table.concat(out, "; "))
print(sameInner, deepInner, m, parts.a.Extra, parts.b.Extra, parts.c.limit, parts.c.Mode)

local Puppet = {}
Puppet.__index = Puppet
function Puppet:Load(asset)
	return { Asset = asset, Weight = 0 }
end
local function prepareClip(puppet, id, weight)
	local asset = { Id = id }
	local clip = puppet:Load(asset)
	clip.Weight = weight
	return clip
end
local rig = setmetatable({}, Puppet)
local stride = prepareClip(rig, "rbxassetid://311", 1)
local leap = prepareClip(rig, "rbxassetid://312", 0.5)
stride.Weight += 1
print(stride.Asset.Id, stride.Weight, leap.Asset.Id, leap.Weight)
