-- Layout under depth: a `(f or g)()` statement after each of 20 nested blocks (`if`,
-- `while`, `repeat ... until`), and 20 nested `("...%x"):format` calls that stay calls.
local fired = 0
local Gate = {}
function Gate.open(k)
	return k % 7 ~= 6 or k < 100
end
local function climb(onDeep, onShallow)
	if Gate.open(0) then
		while Gate.open(3) do
			repeat
				if Gate.open(9) then
					while Gate.open(12) do
						repeat
							if Gate.open(18) then
								while Gate.open(21) do
									repeat
										if Gate.open(27) then
											while Gate.open(30) do
												repeat
													if Gate.open(36) then
														while Gate.open(39) do
															repeat
																if Gate.open(45) then
																	while Gate.open(48) do
																		repeat
																			if Gate.open(54) then
																				while Gate.open(57) do
																					(onDeep or onShallow)(20)
																					break
																				end
																				;(onDeep or onShallow)(19)
																			end
																			;(onDeep or onShallow)(18)
																		until Gate.open(51)
																		;(onDeep or onShallow)(17)
																		break
																	end
																	;(onDeep or onShallow)(16)
																end
																;(onDeep or onShallow)(15)
															until Gate.open(42)
															;(onDeep or onShallow)(14)
															break
														end
														;(onDeep or onShallow)(13)
													end
													;(onDeep or onShallow)(12)
												until Gate.open(33)
												;(onDeep or onShallow)(11)
												break
											end
											;(onDeep or onShallow)(10)
										end
										;(onDeep or onShallow)(9)
									until Gate.open(24)
									;(onDeep or onShallow)(8)
									break
								end
								;(onDeep or onShallow)(7)
							end
							;(onDeep or onShallow)(6)
						until Gate.open(15)
						;(onDeep or onShallow)(5)
						break
					end
					;(onDeep or onShallow)(4)
				end
				;(onDeep or onShallow)(3)
			until Gate.open(6)
			;(onDeep or onShallow)(2)
			break
		end
		;(onDeep or onShallow)(1)
	end
	;(onDeep or onShallow)(0)
end
climb(nil, function(k)
	fired += k + 1
end)
print(fired)
local function stack(seed)
	return ("[%*|%x]"):format(("[%*|%x]"):format(("[%*|%x]"):format(("[%*|%x]"):format(("[%*|%x]"):format(("[%*|%x]"):format(("[%*|%x]"):format(("[%*|%x]"):format(("[%*|%x]"):format(("[%*|%x]"):format(("[%*|%x]"):format(("[%*|%x]"):format(("[%*|%x]"):format(("[%*|%x]"):format(("[%*|%x]"):format(("[%*|%x]"):format(("[%*|%x]"):format(("[%*|%x]"):format(("[%*|%x]"):format(("[%*|%x]"):format(seed, 240), 241), 242), 243), 244), 245), 246), 247), 248), 249), 250), 251), 252), 253), 254), 255), 256), 257), 258), 259)
end
local s = stack("core")
print(#s, string.sub(s, 1, 40), string.sub(s, -12))
