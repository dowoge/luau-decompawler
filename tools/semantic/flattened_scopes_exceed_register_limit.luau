-- Each `do` block frees its local before the next one; kept apart they fit
-- Luau's 255 registers exactly, flattened into one scope they need one more.
local tally = 0
local function fire(...)
	tally += select("#", ...)
end
local function burst(pack, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15, s16, s17, s18, s19, s20, s21, s22, s23, s24, s25, s26, s27, s28, s29, s30, s31, s32, s33, s34, s35, s36, s37, s38, s39, s40, s41, s42, s43, s44, s45, s46, s47, s48, s49, s50, s51, s52, s53, s54, s55, s56, s57, s58, s59, s60, s61, s62, s63, s64, s65, s66, s67, s68, s69, s70, s71, s72, s73, s74, s75, s76, s77, s78, s79, s80, s81, s82, s83, s84, s85, s86, s87, s88, s89, s90, s91, s92, s93, s94, s95, s96, s97, s98, s99, s100, s101, s102, s103, s104, s105, s106, s107, s108, s109, s110, s111, s112, s113, s114, s115, s116, s117, s118, s119, s120, s121, s122, s123, s124, s125, s126, s127, s128, s129, s130, s131, s132, s133, s134, s135, s136, s137, s138, s139, s140, s141, s142, s143, s144, s145, s146, s147, s148, s149, s150)
	do
		local north = pack.north
		fire(north, s1, north, s3, north, s5, north, s7, north, s9, north, s11, north, s13, north, s15, north, s17, north, s19, north, s21, north, s23, north, s25, north, s27, north, s29, north, s31, north, s33, north, s35, north, s37, north, s39, north, s41, north, s43, north, s45, north, s47, north, s49, north, s51, north, s53, north, s55, north, s57, north, s59, north, s61, north, s63, north, s65, north, s67, north, s69, north, s71, north, s73, north, s75, north, s77, north, s79, north, s81, north, s83, north, s85, north, s87, north, s89, north, s91, north, s93, north, s95, north, s97, north, s99, north, s101)
	end
	do
		local south = pack.south
		fire(south, s1, south, s3, south, s5, south, s7, south, s9, south, s11, south, s13, south, s15, south, s17, south, s19, south, s21, south, s23, south, s25, south, s27, south, s29, south, s31, south, s33, south, s35, south, s37, south, s39, south, s41, south, s43, south, s45, south, s47, south, s49, south, s51, south, s53, south, s55, south, s57, south, s59, south, s61, south, s63, south, s65, south, s67, south, s69, south, s71, south, s73, south, s75, south, s77, south, s79, south, s81, south, s83, south, s85, south, s87, south, s89, south, s91, south, s93, south, s95, south, s97, south, s99, south, s101)
	end
	do
		local east = pack.east
		fire(east, s1, east, s3, east, s5, east, s7, east, s9, east, s11, east, s13, east, s15, east, s17, east, s19, east, s21, east, s23, east, s25, east, s27, east, s29, east, s31, east, s33, east, s35, east, s37, east, s39, east, s41, east, s43, east, s45, east, s47, east, s49, east, s51, east, s53, east, s55, east, s57, east, s59, east, s61, east, s63, east, s65, east, s67, east, s69, east, s71, east, s73, east, s75, east, s77, east, s79, east, s81, east, s83, east, s85, east, s87, east, s89, east, s91, east, s93, east, s95, east, s97, east, s99, east, s101)
	end
	return s1, s150
end
print(burst({ north = 1, south = 2, east = 3 }, "first"))
print(tally)
