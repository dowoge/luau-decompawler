-- A helper returning `(string.match(...))` gives one capture and one returning
-- the call gives all; sites taking one or two values must keep the difference.
local function keyOf(line)
	local trimmed = string.gsub(line, "%s", "")
	return (string.match(trimmed, "(%a+)=(%d+)"))
end

local function showPair(line)
	local trimmed = string.gsub(line, "%s", "")
	local k, v = string.match(trimmed, "(%a+)=(%d+)")
	print("pair", k, v)
end

local function parse(line)
	local trimmed = string.gsub(line, "%s", "")
	return string.match(trimmed, "(%a+):(%a+)")
end

local function headOnly(line)
	local head = parse(line)
	return head
end

function bothParts(line)
	local a, b = parse(line)
	print("both", a, b)
end

print(keyOf(" hp = 40 "))
showPair("mp=7")
print(headOnly("team : red"))
bothParts("team : blue")
print(parse("x:y"))
print(select("#", keyOf("a=1")), select("#", parse("a:b")))
