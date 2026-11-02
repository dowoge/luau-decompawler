-- A constructor with bracketed string keys is sized for its own three entries
-- and grows on the stores after it; pairs order depends on that growth.
local function dump(t)
	local parts = {}
	for key, value in pairs(t) do
		parts[#parts + 1] = key .. "=" .. value
	end
	return table.concat(parts, ",")
end

local theme = { ["Shadow"] = "gray", ["Border"] = "black", ["Glow"] = "white" }
theme.Text = "ivory"
theme.Hover = "teal"
theme.Pressed = "navy"
theme.Disabled = "slate"
theme.Focus = "cyan"
print(dump(theme))
