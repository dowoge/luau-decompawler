-- Local helpers and function literals called with functions: at -O2 the
-- compiler copies the helper in and, where the argument is a local function,
-- that function too, so the copy holds copies. A helper may also call a
-- function literal it could not fold in its own code, which the caller's
-- copy has folded. Each comes back as the call it was.

local seen = {}
local function note(s)
	seen[#seen + 1] = s
	return #seen
end

local callback = function()
	return "final " .. tostring(#"abc")
end

local value = (function(cb)
	while (function()
		local n = 5
		return n == (function()
			return 5
		end)()
	end)() do
		if cb() then
			return (function(cb2)
				if cb2() then
					return cb2() .. "!"
				end
				return "fail inner"
			end)(cb)
		end
		return "fail T2"
	end
	return "fail T1"
end)(callback)
print("out=", value)

local function protected(body, cleanup)
	local ok, err = pcall(body)
	cleanup()
	if not ok then
		error(err, 0)
	end
end

local state = { level = 1 }
local function run(n)
	local saved = state.level
	protected(function()
		state.level = n
		note("body " .. n)
		if n > 2 then
			error("too high")
		end
	end, function()
		state.level = saved
		note("cleanup " .. n)
	end)
	return state.level
end
print(run(2), pcall(run, 3), state.level)

local function twice(f, x)
	return f(f(x))
end
local inc = function(v)
	return v + 1
end
print(twice(inc, 1), twice(function(v)
	return v * 3
end, 2))
print(table.concat(seen, ","))
