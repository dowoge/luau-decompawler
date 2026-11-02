-- `setfenv` gives a helper its own `warn`: the same call in the helper and in
-- the chunk resolve differently, so neither may be merged or imported.
local function alert(msg)
	warn("alert", msg)
end
setfenv(alert, {
	warn = function(...)
		report("[env]", ...)
	end,
})
report = print
warn = function(...)
	print("[global]", ...)
end
warn("alert", "low health")
alert("low health")
