-- A function near the 200-local limit passes arguments that hold `and`/`or`
-- choices between calls with effects; the decompiled form must still compile and agree.
local log = {}
local function note(tag, v)
	log[#log + 1] = tag
	return v
end
local function count(...)
	return select("#", ...)
end
local function mix(r1, r2, r3, r4, r5, r6, r7, r8, r9, r10, r11, r12, r13, r14, r15, r16, r17, r18, r19, r20, r21, r22, r23, r24, r25, r26, r27, r28, r29, r30, r31, r32, r33, r34, r35, r36, r37, r38, r39, r40, r41, r42, r43, r44, r45, r46, r47, r48, r49, r50, r51, r52, r53, r54, r55, r56, r57, r58, r59, r60, r61, r62, r63, r64, r65, r66, r67, r68, r69, r70, r71, r72, r73, r74, r75, r76, r77, r78, r79, r80, r81, r82, r83, r84, r85, r86, r87, r88, r89, r90, r91, r92, r93, r94, r95, r96, r97, r98, r99, r100, r101, r102, r103, r104, r105, r106, r107, r108, r109, r110, r111, r112, r113, r114, r115, r116, r117, r118, r119, r120, r121, r122, r123, r124, r125, r126, r127, r128, r129, r130, r131, r132, r133, r134, r135, r136, r137, r138, r139, r140, r141, r142, r143, r144, r145, r146, r147, r148, r149, r150, r151, r152, r153, r154, r155, r156, r157, r158, r159, r160, r161, r162, r163, r164, r165, r166, r167, r168, r169, r170, r171, r172, r173, r174, r175, r176, r177, r178, r179, r180, r181, r182, r183, r184, r185)
	local total = 0
	for pass = 1, 2 do
		total += count(note("c0a0", r1), (r8 and r12 or r6), (pass > 1 and note("c0b2", r23) or r11), (r22 or note("c0d3", r16)), note("c0a4", r29), (r36 and r56 or r26), (pass > 1 and note("c0b6", r67) or r31), (r50 or note("c0d7", r36)), note("c0a8", r57), (r64 and r100 or r46), (pass > 1 and note("c0b10", r111) or r51), (r78 or note("c0d11", r56)))
		total += count(note("c1a0", r32), (r39 and r29 or r19), (pass > 1 and note("c1b2", r40) or r24), (r53 or note("c1d3", r29)), note("c1a4", r60), (r67 and r73 or r39), (pass > 1 and note("c1b6", r84) or r44), (r81 or note("c1d7", r49)), note("c1a8", r88), (r95 and r117 or r59), (pass > 1 and note("c1b10", r128) or r64), (r109 or note("c1d11", r69)))
		total += count(note("c2a0", r63), (r70 and r46 or r32), (pass > 1 and note("c2b2", r57) or r37), (r84 or note("c2d3", r42)), note("c2a4", r91), (r98 and r90 or r52), (pass > 1 and note("c2b6", r101) or r57), (r112 or note("c2d7", r62)), note("c2a8", r119), (r126 and r134 or r72), (pass > 1 and note("c2b10", r145) or r77), (r140 or note("c2d11", r82)))
		total += count(note("c3a0", r94), (r101 and r63 or r45), (pass > 1 and note("c3b2", r74) or r50), (r115 or note("c3d3", r55)), note("c3a4", r122), (r129 and r107 or r65), (pass > 1 and note("c3b6", r118) or r70), (r143 or note("c3d7", r75)), note("c3a8", r150), (r157 and r151 or r85), (pass > 1 and note("c3b10", r162) or r90), (r171 or note("c3d11", r95)))
		total += count(note("c4a0", r125), (r132 and r80 or r58), (pass > 1 and note("c4b2", r91) or r63), (r146 or note("c4d3", r68)), note("c4a4", r153), (r160 and r124 or r78), (pass > 1 and note("c4b6", r135) or r83), (r174 or note("c4d7", r88)), note("c4a8", r181), (r3 and r168 or r98), (pass > 1 and note("c4b10", r179) or r103), (r17 or note("c4d11", r108)))
		total += count(note("c5a0", r156), (r163 and r97 or r71), (pass > 1 and note("c5b2", r108) or r76), (r177 or note("c5d3", r81)), note("c5a4", r184), (r6 and r141 or r91), (pass > 1 and note("c5b6", r152) or r96), (r20 or note("c5d7", r101)), note("c5a8", r27), (r34 and r185 or r111), (pass > 1 and note("c5b10", r11) or r116), (r48 or note("c5d11", r121)))
	end
	return total
end
local args = {}
for i = 1, 185 do
	args[i] = if i % 4 == 0 then false else i
end
print(mix(table.unpack(args)))
print(#log, log[1], log[#log])
