-- Layout pressure: long calls nested deep in loops, constructors whose items span lines
-- (one item or several), and blank lines between calls; the program must still run the same.
function Offset(x, y, z)
	return { X = x, Y = y, Z = z }
end
local function Remap(set, fn)
	local out = {}
	for k, v in set do
		local nk, nv = fn(k, v)
		out[nk] = nv
	end
	return out
end
local function placeDecals(normal, decals)
	if normal then
		for _, decal in decals do
			if decal.Visible then
				for _ = 1, 1 do
					local pos = Offset(normal.X * decal.Depth, 0, normal.Z * decal.Depth)
					local text = string.sub(decal.Label .. "_decal_text", decal.From, decal.To)
					print(pos.X, pos.Y, pos.Z, text)
				end
			end
		end
	end
end
placeDecals({ X = 1, Z = 4 }, { { Visible = true, Depth = 3, Label = "warning", From = 3, To = 9 } })
local renamed = Remap({ hp = 10 }, function(key, value)
	return "max" .. key, value * 3
end)
print(renamed.maxhp)
print(("%s|%s|%s"):format(string.rep("q", 44), string.rep("r", 44), string.rep("s", 44)))

local function widget(kind)
	local made = 0
	return function(fields)
		made += 1
		fields.Kind = kind
		fields.Serial = made
		return fields
	end
end
local function render(list)
	local rows = {}
	for index, entry in ipairs(list) do
		local names = {}
		for name in pairs(entry) do
			names[#names + 1] = name
		end
		table.sort(names, function(a, b)
			return a > b
		end)
		local text = {}
		for _, name in ipairs(names) do
			text[#text + 1] = string.format("%s:%s", name, tostring(entry[name]))
		end
		rows[index] = "<" .. table.concat(text, ";") .. ">"
	end
	return table.concat(rows, " / ")
end
local panel = {
	widget("Gauge")({
		Caption = "Fuel",
		Fill = 0.75,
		Order = 2,
		Pulse = true,
		Tint = "amber",
	}),
	widget("Toggle")({
		Caption = "Lights",
		Order = 7,
		Pulse = false,
		Group = 3,
	}),
}
local lone = {
	widget("Grid")({
		Columns = 4,
		Gap = 2,
		Flow = "Rows",
		Clip = true,
	}),
}
print(render(panel))
print(render(lone))

local function report()
	print("first line")

	print("after a gap")
	print("right after")
end
local function reportLater()
	return function()
		print("inner first")

		print("inner after gap")
	end
end
report()
reportLater()()
return report, reportLater
