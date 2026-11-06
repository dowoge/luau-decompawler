-- an interpolation hole whose expression starts with a table constructor
local x = tonumber("1")
local t = {k = 2}
print(`{ {} ~= nil }`, `{ {x} ~= nil }`, `{ {a = 1} ~= nil }`, `{ { {} } ~= nil }`)
print(`<{ ({x, x})[2] + t.k }>`, `{x == {}}`, `{ #{1, 2, x} }`)
