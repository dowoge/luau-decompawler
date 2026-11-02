-- A call in a keyed entry is cut to one value; a dynamic key among list items
-- lands in the array or hash part by its value, and layout decides pairs order.
local function dump(t)
	local parts = {}
	for k, v in pairs(t) do
		table.insert(parts, tostring(k) .. "=" .. tostring(v))
	end
	return table.concat(parts, ",")
end

local function pair()
	print("pair called")
	return nil, "extra"
end
local t = { [1] = "a", [2] = "b", [3] = pair() }
print(dump(t))
t.Alpha = 1
t.Beta = 2
t.Gamma = 3
t[25] = 25
print(dump(t))

local function mixed(k)
	return { "first", [k] = nil, "third" }
end
for k = 1, 3 do
	local m = mixed(k)
	print(dump(m))
	m.Alpha = 1
	m.Beta = 2
	m[25] = 25
	print(dump(m))
end

local function note(v)
	print("note", v)
	return v
end
local function nothing()
	return
end
local function build(k, tail)
	return { [k] = "keyed", if k == 2 then note("two") else note("other"), tail() }
end
for _, k in ipairs({ 1, 2, 3, "tag" }) do
	print(dump(build(k, nothing)))
end
