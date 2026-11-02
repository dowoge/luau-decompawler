-- `("...%*"):format(v)` as a call argument, constructor item, assignment or
-- return, cut to one value or left open, keeps its text and its value count.
local sent = {}
local function send(text)
	sent[#sent + 1] = text
end
local function two()
	return "left", "right"
end

local Notify = {}
function Notify.warn(name)
	local text = ("Missing asset %*"):format(name)
	send(text)
	return #sent
end
function Notify.banner(name)
	return { ("Welcome %*"):format(name), ("v%*"):format(2) }
end
function Notify.raw(name)
	return ("raw:%*"):format(name)
end
function Notify.split(name)
	local a, b = ("%* split"):format(name)
	return tostring(a) .. "|" .. tostring(b)
end
function Notify.joined()
	return ("%* & %*"):format(two())
end
function Notify.legacy(name)
	return (("old %s"):format(name))
end
function Notify.measure(name)
	print(#("<%*>"):format(name), string.len(("[%*]"):format(name)))
end

print(Notify.warn("sword"), Notify.warn(3))
local b = Notify.banner("ana")
print(#b, b[1], b[2])
print(select("#", Notify.raw("x")), Notify.raw(true))
print(Notify.split("y"), Notify.joined(), Notify.legacy("z"))
Notify.measure("abc")
send(("direct %*"):format("arg"))
table.insert(sent, (("boxed %*"):format(9)))
print(("print %*"):format(1.5))
print(#sent, table.concat(sent, "; "))
