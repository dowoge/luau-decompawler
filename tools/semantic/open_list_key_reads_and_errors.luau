-- Constructor keys are read when Luau reads them (an upvalue before the value's
-- callback, a register at the store), and bad keys raise before later entries run.
local function call(cb, ...)
	return cb(...)
end
local function trailing()
	print("trailing")
	return 31, nil, 33
end
local function later()
	print("later")
	return 4
end

local function viaUpvalue(key, after, throws)
	return function(flag, call, trailing)
		local function shift()
			print("shift", key, after)
			key = after
			if throws then
				error("shift failed")
			end
			return 8, 88
		end
		return { [key] = call(shift), late = call(later), if flag then call(later) else call(shift), trailing() }
	end
end
local function upvalueCase(key, after, throws, flag, call, trailing)
	return call(viaUpvalue(key, after, throws), flag, call, trailing)
end

local function ownRegister(key, after, throws, flag, call, trailing)
	local function shift()
		print("shift", key, after)
		key = after
		if throws then
			error("shift failed")
		end
		return 8, 88
	end
	return { [key] = call(shift), late = call(later), if flag then call(later) else call(shift), trailing() }
end

local function viaIndex(before, after, throws, flag, call, trailing)
	local key = before
	local function shift()
		print("shift", key, after)
		key = after
		if throws then
			error("shift failed")
		end
		return 8, 88
	end
	local probe = setmetatable({}, { __index = function() return shift() end })
	return { first = call(later), [key] = probe.anything, late = call(later), if flag then call(later) else call(shift), trailing() }
end

local function viaCopy(key, after, throws, flag, call, trailing)
	local function shift()
		print("shift", key, after)
		key = after
		if throws then
			error("shift failed")
		end
		return 8, 88
	end
	local copy = key
	return { [copy] = call(shift), late = call(later), if flag then call(later) else call(shift), trailing() }
end

local function viaOr(before, after, throws, flag, call, trailing)
	local key = before
	local function shift()
		print("shift", key, after)
		key = after
		if throws then
			error("shift failed")
		end
		return 8, 88
	end
	return { [key or "start"] = call(shift), late = call(later), if flag then call(later) else call(shift), trailing() }
end

local function run(build, label, before, after, throws, flag)
	print("case", label)
	local ok, result = pcall(build, before, after, throws, flag, call, trailing)
	if ok then
		print("table", result.start, result.done, result[false], result[1], result[2], result[3], result[4], #result)
	else
		print("error", string.match(result, ":%d+: (.*)$") or result)
	end
end

local cases = {
	{ "rename", "start", "done", false, true },
	{ "from nil", nil, "done", false, false },
	{ "from nan", 0 / 0, "done", false, true },
	{ "to nil", "start", nil, false, true },
	{ "to nan", "start", 0 / 0, false, false },
	{ "to false", "start", false, false, true },
	{ "raises", "start", "done", true, false },
}
for index, build in { upvalueCase, ownRegister, viaIndex, viaCopy, viaOr } do
	print("builder", index)
	for _, c in cases do
		run(build, c[1], c[2], c[3], c[4], c[5])
	end
end

local trail = {}
local function track(label, value)
	trail[#trail + 1] = label
	if value == "boom" then
		error(label)
	end
	return value
end
local function forward(...)
	track("forward", nil)
	return ...
end
local function make(track, forward, key, value, flag, ...)
	return {
		[track("key", key)] = track("first", value),
		second = track("second", nil),
		[key] = track("again", value),
		if flag then track("yes", 11) else track("no", 12),
		forward(...),
	}
end
local function attempt(key, value, flag)
	trail = {}
	local ok, result = pcall(make, track, forward, key, value, flag, 5, nil, 7)
	print(ok, table.concat(trail, ","))
	if ok then
		print("length", #result)
		for k, v in next, result do
			print(type(k), k, v)
		end
	else
		print("error", string.match(result, ":%d+: (.*)") or result)
	end
end
attempt("extra", 9, true)
attempt(false, nil, false)
attempt(2, false, true)
attempt(nil, 9, true)
attempt(0 / 0, 9, false)
attempt("extra", "boom", true)
