-- Immediately called functions given a callback, seven deep: past
-- LuauCompileInlineDepth the copies keep real calls where the helpers' own
-- code, copied from less deep, has copies, and the other way round; and the
-- callback itself is copied in where it is called, also from a closure
-- capturing it.

local calls = 0
local callback = function()
	calls += 1
	return "final " .. calls
end
local cb = callback
local value = (function(cb)
	while (function() local n = 5 return n == (function() return 5, 6 end)() end)() do
		if (function() return cb() end)() then
			return (function(cb)
				while (function() local n = 5 return n == (function() return 5, 6 end)() end)() do
					if (function() return cb() end)() then
						return (function(cb)
							while (function() local n = 5 return n == (function() return 5, 6 end)() end)() do
								if (function() return cb() end)() then
									return (function(cb)
										while (function() local n = 5 return n == (function() return 5, 6 end)() end)() do
											if (function() return cb() end)() then
												return (function(cb)
													while (function() local n = 5 return n == (function() return 5, 6 end)() end)() do
														if (function() return cb() end)() then
															return (function(cb)
																while (function() local n = 5 return n == (function() return 5, 6 end)() end)() do
																	if (function() return cb() end)() then
																		return (function(cb)
																			while (function() local n = 5 return n == (function() return 5, 6 end)() end)() do
																				if (function() return cb() end)() then
																					return 																					cb()
																				end
																				return (function() return "Fail T2" end)()
																			end
																			return (function() return "Fail T1" end)()
																		end)(cb)
																	end
																	return (function() return "Fail T2" end)()
																end
																return (function() return "Fail T1" end)()
															end)(cb)
														end
														return (function() return "Fail T2" end)()
													end
													return (function() return "Fail T1" end)()
												end)(cb)
											end
											return (function() return "Fail T2" end)()
										end
										return (function() return "Fail T1" end)()
									end)(cb)
								end
								return (function() return "Fail T2" end)()
							end
							return (function() return "Fail T1" end)()
						end)(cb)
					end
					return (function() return "Fail T2" end)()
				end
				return (function() return "Fail T1" end)()
			end)(cb)
		end
		return (function() return "Fail T2" end)()
	end
	return (function() return "Fail T1" end)()
end)(cb)
print("out=", value, calls)
