-- Strings with newlines, patterns or format directives keep their bytes in any
-- quoting, including table keys printed next to index brackets.
local notice = "Your inventory is full right now.\nSell or drop something first,\nthen try picking it up again."
local padded = "\nStack trace begins:\n\n"
local grab = "^.-\n(.-)\n"
local template = "Wanted %s\ngot %s\nin slot %d"
local tiny = "x\ny\nz"
local closer = "first ]] part\nsecond ]=] part\nthird part here"

print(string.match("one\ntwo\nthree", grab))
print(#notice, notice)
print(#padded, padded)
print(template:format("sword", "shield", 4))
print(#tiny, tiny)
print(#closer, closer)

local function keyed(t)
	t["line one\nline two\n"] = 5
	return t["line one\nline two\n"], ({ ["]]\nedge\n"] = 6 })["]]\nedge\n"]
end
print(keyed({}))
