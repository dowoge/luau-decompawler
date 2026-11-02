-- A value made by `a or b` between the tests of one `if`, read again after
-- the test that first reads it: it stays a variable of its own, or the later
-- reads name one never declared.
local function code_of(text)
	if type(text) == "number" then
		return text
	end
	if type(text) == "string" then
		local digits = string.match(text, "^#(%d+)$") or string.match(text, "^code:(%d+)$")
		if digits ~= nil and tonumber(digits) > 9 then
			return tonumber(digits) * 2, digits
		end
	end
	return nil
end

local function label(item)
	if item.kind == "box" then
		local name = item.name or item.fallback
		if name ~= "" and #name < 8 then
			return name .. "!", #name
		end
	end
	return "?"
end

local lookup = {code_of = code_of, label = label}
for _, v in {"#42", "code:7", "code:123", "x", 5, "#"} do
	print(lookup.code_of(v))
end
for _, item in {{kind = "box", name = "lid"}, {kind = "box", fallback = "crate"}, {kind = "box", name = ""}, {kind = "bag"}} do
	print(lookup.label(item))
end
