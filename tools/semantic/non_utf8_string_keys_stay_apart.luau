-- Keys whose bytes are not UTF-8 are as many different keys as they are
-- different byte strings. Once written with U+FFFD for each bad byte, they
-- all came out as one key (`t["\239\191\189\239\191\189"]`), each value
-- landing on the others'. A valid character among the bytes kept for raw
-- ones (U+10FF00 and up) is itself, not a raw byte.

local t = {}
t["\216\245"] = 1
t["\201\200"] = 2
t["\255"] = 3
t["\255\254"] = 4
print(t["\216\245"], t["\201\200"], t["\255"], t["\255\254"])

local c = { ["\216\245"] = "a", ["\201\200"] = "b", ["ok\128x"] = "c" }
print(c["\216\245"], c["\201\200"], c["ok\128x"], c["\216"])

-- the obfuscators' cache: one value per key, computed once
local cache = {}
local function get(k, v)
	local got = cache[k]
	if not got then
		got = v
		cache[k] = got
	end
	return got
end
print(get("\216\245\180\200", 11), get("\201\200\180\200", 22), get("\216\245\180\200", 33))

t["\244\143\188\128"] = "valid U+10FF00"
t["\128"] = "raw 128"
print(t["\244\143\188\128"], t["\128"], #"\244\143\188\128")

-- as fields read through a method call and a nested table
local obj = { ["\200\201"] = { ["\202"] = function(self, x) return x * 2 end } }
print(obj["\200\201"]["\202"](nil, 21))
