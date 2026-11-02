-- A comparison tree some eight levels deep, inside two loops: every leaf
-- folds into one running total or a table of cells shared by all of them.
local cells = {}
for i = 1, 48 do
	cells[i] = i % 5 + 1
end
local function run(codes, rounds)
	local total, step = 7, 0
	repeat
		step += 1
		for _, code in ipairs(codes) do
			local key = (code * 31 + step) % 192
			if key < 96 then
				if key < 48 then
					if key < 24 then
						if key < 12 then
							if key < 6 then
								if key < 3 then
									if key < 1 then
										total = (total * 3 + cells[1]) % 10007
									else
										if key < 2 then
											cells[6] = (cells[6] + total + 1) % 211
										else
											total = (total + 2 * step) % 10007
											cells[3] += 1
										end
									end
								else
									if key < 4 then
										if total % 2 == 0 then
											total += 3
										else
											total -= 3
										end
									else
										if key < 5 then
											total = (total * 3 + cells[5]) % 10007
										else
											cells[26] = (cells[26] + total + 5) % 211
										end
									end
								end
							else
								if key < 9 then
									if key < 7 then
										total = (total + 6 * step) % 10007
										cells[7] += 1
									else
										if key < 8 then
											if total % 2 == 0 then
												total += 7
											else
												total -= 7
											end
										else
											total = (total * 3 + cells[9]) % 10007
										end
									end
								else
									if key < 10 then
										cells[46] = (cells[46] + total + 9) % 211
									else
										if key < 11 then
											total = (total + 10 * step) % 10007
											cells[11] += 1
										else
											if total % 2 == 0 then
												total += 11
											else
												total -= 11
											end
										end
									end
								end
							end
						else
							if key < 18 then
								if key < 15 then
									if key < 13 then
										total = (total * 3 + cells[13]) % 10007
									else
										if key < 14 then
											cells[18] = (cells[18] + total + 13) % 211
										else
											total = (total + 14 * step) % 10007
											cells[15] += 1
										end
									end
								else
									if key < 16 then
										if total % 2 == 0 then
											total += 15
										else
											total -= 2
										end
									else
										if key < 17 then
											total = (total * 3 + cells[17]) % 10007
										else
											cells[38] = (cells[38] + total + 17) % 211
										end
									end
								end
							else
								if key < 21 then
									if key < 19 then
										total = (total + 18 * step) % 10007
										cells[19] += 1
									else
										if key < 20 then
											if total % 2 == 0 then
												total += 19
											else
												total -= 6
											end
										else
											total = (total * 3 + cells[21]) % 10007
										end
									end
								else
									if key < 22 then
										cells[10] = (cells[10] + total + 21) % 211
									else
										if key < 23 then
											total = (total + 22 * step) % 10007
											cells[23] += 1
										else
											if total % 2 == 0 then
												total += 23
											else
												total -= 10
											end
										end
									end
								end
							end
						end
					else
						if key < 36 then
							if key < 30 then
								if key < 27 then
									if key < 25 then
										total = (total * 3 + cells[25]) % 10007
									else
										if key < 26 then
											cells[30] = (cells[30] + total + 25) % 211
										else
											total = (total + 26 * step) % 10007
											cells[27] += 1
										end
									end
								else
									if key < 28 then
										if total % 2 == 0 then
											total += 27
										else
											total -= 1
										end
									else
										if key < 29 then
											total = (total * 3 + cells[29]) % 10007
										else
											cells[2] = (cells[2] + total + 29) % 211
										end
									end
								end
							else
								if key < 33 then
									if key < 31 then
										total = (total + 30 * step) % 10007
										cells[31] += 1
									else
										if key < 32 then
											if total % 2 == 0 then
												total += 31
											else
												total -= 5
											end
										else
											total = (total * 3 + cells[33]) % 10007
										end
									end
								else
									if key < 34 then
										cells[22] = (cells[22] + total + 33) % 211
									else
										if key < 35 then
											total = (total + 34 * step) % 10007
											cells[35] += 1
										else
											if total % 2 == 0 then
												total += 35
											else
												total -= 9
											end
										end
									end
								end
							end
						else
							if key < 42 then
								if key < 39 then
									if key < 37 then
										total = (total * 3 + cells[37]) % 10007
									else
										if key < 38 then
											cells[42] = (cells[42] + total + 37) % 211
										else
											total = (total + 38 * step) % 10007
											cells[39] += 1
										end
									end
								else
									if key < 40 then
										if total % 2 == 0 then
											total += 39
										else
											total -= 0
										end
									else
										if key < 41 then
											total = (total * 3 + cells[41]) % 10007
										else
											cells[14] = (cells[14] + total + 41) % 211
										end
									end
								end
							else
								if key < 45 then
									if key < 43 then
										total = (total + 42 * step) % 10007
										cells[43] += 1
									else
										if key < 44 then
											if total % 2 == 0 then
												total += 43
											else
												total -= 4
											end
										else
											total = (total * 3 + cells[45]) % 10007
										end
									end
								else
									if key < 46 then
										cells[34] = (cells[34] + total + 45) % 211
									else
										if key < 47 then
											total = (total + 46 * step) % 10007
											cells[47] += 1
										else
											if total % 2 == 0 then
												total += 47
											else
												total -= 8
											end
										end
									end
								end
							end
						end
					end
				else
					if key < 72 then
						if key < 60 then
							if key < 54 then
								if key < 51 then
									if key < 49 then
										total = (total * 3 + cells[1]) % 10007
									else
										if key < 50 then
											cells[6] = (cells[6] + total + 49) % 211
										else
											total = (total + 50 * step) % 10007
											cells[3] += 1
										end
									end
								else
									if key < 52 then
										if total % 2 == 0 then
											total += 51
										else
											total -= 12
										end
									else
										if key < 53 then
											total = (total * 3 + cells[5]) % 10007
										else
											cells[26] = (cells[26] + total + 53) % 211
										end
									end
								end
							else
								if key < 57 then
									if key < 55 then
										total = (total + 54 * step) % 10007
										cells[7] += 1
									else
										if key < 56 then
											if total % 2 == 0 then
												total += 55
											else
												total -= 3
											end
										else
											total = (total * 3 + cells[9]) % 10007
										end
									end
								else
									if key < 58 then
										cells[46] = (cells[46] + total + 57) % 211
									else
										if key < 59 then
											total = (total + 58 * step) % 10007
											cells[11] += 1
										else
											if total % 2 == 0 then
												total += 59
											else
												total -= 7
											end
										end
									end
								end
							end
						else
							if key < 66 then
								if key < 63 then
									if key < 61 then
										total = (total * 3 + cells[13]) % 10007
									else
										if key < 62 then
											cells[18] = (cells[18] + total + 61) % 211
										else
											total = (total + 62 * step) % 10007
											cells[15] += 1
										end
									end
								else
									if key < 64 then
										if total % 2 == 0 then
											total += 63
										else
											total -= 11
										end
									else
										if key < 65 then
											total = (total * 3 + cells[17]) % 10007
										else
											cells[38] = (cells[38] + total + 65) % 211
										end
									end
								end
							else
								if key < 69 then
									if key < 67 then
										total = (total + 66 * step) % 10007
										cells[19] += 1
									else
										if key < 68 then
											if total % 2 == 0 then
												total += 67
											else
												total -= 2
											end
										else
											total = (total * 3 + cells[21]) % 10007
										end
									end
								else
									if key < 70 then
										cells[10] = (cells[10] + total + 69) % 211
									else
										if key < 71 then
											total = (total + 70 * step) % 10007
											cells[23] += 1
										else
											if total % 2 == 0 then
												total += 71
											else
												total -= 6
											end
										end
									end
								end
							end
						end
					else
						if key < 84 then
							if key < 78 then
								if key < 75 then
									if key < 73 then
										total = (total * 3 + cells[25]) % 10007
									else
										if key < 74 then
											cells[30] = (cells[30] + total + 73) % 211
										else
											total = (total + 74 * step) % 10007
											cells[27] += 1
										end
									end
								else
									if key < 76 then
										if total % 2 == 0 then
											total += 75
										else
											total -= 10
										end
									else
										if key < 77 then
											total = (total * 3 + cells[29]) % 10007
										else
											cells[2] = (cells[2] + total + 77) % 211
										end
									end
								end
							else
								if key < 81 then
									if key < 79 then
										total = (total + 78 * step) % 10007
										cells[31] += 1
									else
										if key < 80 then
											if total % 2 == 0 then
												total += 79
											else
												total -= 1
											end
										else
											total = (total * 3 + cells[33]) % 10007
										end
									end
								else
									if key < 82 then
										cells[22] = (cells[22] + total + 81) % 211
									else
										if key < 83 then
											total = (total + 82 * step) % 10007
											cells[35] += 1
										else
											if total % 2 == 0 then
												total += 83
											else
												total -= 5
											end
										end
									end
								end
							end
						else
							if key < 90 then
								if key < 87 then
									if key < 85 then
										total = (total * 3 + cells[37]) % 10007
									else
										if key < 86 then
											cells[42] = (cells[42] + total + 85) % 211
										else
											total = (total + 86 * step) % 10007
											cells[39] += 1
										end
									end
								else
									if key < 88 then
										if total % 2 == 0 then
											total += 87
										else
											total -= 9
										end
									else
										if key < 89 then
											total = (total * 3 + cells[41]) % 10007
										else
											cells[14] = (cells[14] + total + 89) % 211
										end
									end
								end
							else
								if key < 93 then
									if key < 91 then
										total = (total + 90 * step) % 10007
										cells[43] += 1
									else
										if key < 92 then
											if total % 2 == 0 then
												total += 91
											else
												total -= 0
											end
										else
											total = (total * 3 + cells[45]) % 10007
										end
									end
								else
									if key < 94 then
										cells[34] = (cells[34] + total + 93) % 211
									else
										if key < 95 then
											total = (total + 94 * step) % 10007
											cells[47] += 1
										else
											if total % 2 == 0 then
												total += 95
											else
												total -= 4
											end
										end
									end
								end
							end
						end
					end
				end
			else
				if key < 144 then
					if key < 120 then
						if key < 108 then
							if key < 102 then
								if key < 99 then
									if key < 97 then
										total = (total * 3 + cells[1]) % 10007
									else
										if key < 98 then
											cells[6] = (cells[6] + total + 97) % 211
										else
											total = (total + 98 * step) % 10007
											cells[3] += 1
										end
									end
								else
									if key < 100 then
										if total % 2 == 0 then
											total += 99
										else
											total -= 8
										end
									else
										if key < 101 then
											total = (total * 3 + cells[5]) % 10007
										else
											cells[26] = (cells[26] + total + 101) % 211
										end
									end
								end
							else
								if key < 105 then
									if key < 103 then
										total = (total + 102 * step) % 10007
										cells[7] += 1
									else
										if key < 104 then
											if total % 2 == 0 then
												total += 103
											else
												total -= 12
											end
										else
											total = (total * 3 + cells[9]) % 10007
										end
									end
								else
									if key < 106 then
										cells[46] = (cells[46] + total + 105) % 211
									else
										if key < 107 then
											total = (total + 106 * step) % 10007
											cells[11] += 1
										else
											if total % 2 == 0 then
												total += 107
											else
												total -= 3
											end
										end
									end
								end
							end
						else
							if key < 114 then
								if key < 111 then
									if key < 109 then
										total = (total * 3 + cells[13]) % 10007
									else
										if key < 110 then
											cells[18] = (cells[18] + total + 109) % 211
										else
											total = (total + 110 * step) % 10007
											cells[15] += 1
										end
									end
								else
									if key < 112 then
										if total % 2 == 0 then
											total += 111
										else
											total -= 7
										end
									else
										if key < 113 then
											total = (total * 3 + cells[17]) % 10007
										else
											cells[38] = (cells[38] + total + 113) % 211
										end
									end
								end
							else
								if key < 117 then
									if key < 115 then
										total = (total + 114 * step) % 10007
										cells[19] += 1
									else
										if key < 116 then
											if total % 2 == 0 then
												total += 115
											else
												total -= 11
											end
										else
											total = (total * 3 + cells[21]) % 10007
										end
									end
								else
									if key < 118 then
										cells[10] = (cells[10] + total + 117) % 211
									else
										if key < 119 then
											total = (total + 118 * step) % 10007
											cells[23] += 1
										else
											if total % 2 == 0 then
												total += 119
											else
												total -= 2
											end
										end
									end
								end
							end
						end
					else
						if key < 132 then
							if key < 126 then
								if key < 123 then
									if key < 121 then
										total = (total * 3 + cells[25]) % 10007
									else
										if key < 122 then
											cells[30] = (cells[30] + total + 121) % 211
										else
											total = (total + 122 * step) % 10007
											cells[27] += 1
										end
									end
								else
									if key < 124 then
										if total % 2 == 0 then
											total += 123
										else
											total -= 6
										end
									else
										if key < 125 then
											total = (total * 3 + cells[29]) % 10007
										else
											cells[2] = (cells[2] + total + 125) % 211
										end
									end
								end
							else
								if key < 129 then
									if key < 127 then
										total = (total + 126 * step) % 10007
										cells[31] += 1
									else
										if key < 128 then
											if total % 2 == 0 then
												total += 127
											else
												total -= 10
											end
										else
											total = (total * 3 + cells[33]) % 10007
										end
									end
								else
									if key < 130 then
										cells[22] = (cells[22] + total + 129) % 211
									else
										if key < 131 then
											total = (total + 130 * step) % 10007
											cells[35] += 1
										else
											if total % 2 == 0 then
												total += 131
											else
												total -= 1
											end
										end
									end
								end
							end
						else
							if key < 138 then
								if key < 135 then
									if key < 133 then
										total = (total * 3 + cells[37]) % 10007
									else
										if key < 134 then
											cells[42] = (cells[42] + total + 133) % 211
										else
											total = (total + 134 * step) % 10007
											cells[39] += 1
										end
									end
								else
									if key < 136 then
										if total % 2 == 0 then
											total += 135
										else
											total -= 5
										end
									else
										if key < 137 then
											total = (total * 3 + cells[41]) % 10007
										else
											cells[14] = (cells[14] + total + 137) % 211
										end
									end
								end
							else
								if key < 141 then
									if key < 139 then
										total = (total + 138 * step) % 10007
										cells[43] += 1
									else
										if key < 140 then
											if total % 2 == 0 then
												total += 139
											else
												total -= 9
											end
										else
											total = (total * 3 + cells[45]) % 10007
										end
									end
								else
									if key < 142 then
										cells[34] = (cells[34] + total + 141) % 211
									else
										if key < 143 then
											total = (total + 142 * step) % 10007
											cells[47] += 1
										else
											if total % 2 == 0 then
												total += 143
											else
												total -= 0
											end
										end
									end
								end
							end
						end
					end
				else
					if key < 168 then
						if key < 156 then
							if key < 150 then
								if key < 147 then
									if key < 145 then
										total = (total * 3 + cells[1]) % 10007
									else
										if key < 146 then
											cells[6] = (cells[6] + total + 145) % 211
										else
											total = (total + 146 * step) % 10007
											cells[3] += 1
										end
									end
								else
									if key < 148 then
										if total % 2 == 0 then
											total += 147
										else
											total -= 4
										end
									else
										if key < 149 then
											total = (total * 3 + cells[5]) % 10007
										else
											cells[26] = (cells[26] + total + 149) % 211
										end
									end
								end
							else
								if key < 153 then
									if key < 151 then
										total = (total + 150 * step) % 10007
										cells[7] += 1
									else
										if key < 152 then
											if total % 2 == 0 then
												total += 151
											else
												total -= 8
											end
										else
											total = (total * 3 + cells[9]) % 10007
										end
									end
								else
									if key < 154 then
										cells[46] = (cells[46] + total + 153) % 211
									else
										if key < 155 then
											total = (total + 154 * step) % 10007
											cells[11] += 1
										else
											if total % 2 == 0 then
												total += 155
											else
												total -= 12
											end
										end
									end
								end
							end
						else
							if key < 162 then
								if key < 159 then
									if key < 157 then
										total = (total * 3 + cells[13]) % 10007
									else
										if key < 158 then
											cells[18] = (cells[18] + total + 157) % 211
										else
											total = (total + 158 * step) % 10007
											cells[15] += 1
										end
									end
								else
									if key < 160 then
										if total % 2 == 0 then
											total += 159
										else
											total -= 3
										end
									else
										if key < 161 then
											total = (total * 3 + cells[17]) % 10007
										else
											cells[38] = (cells[38] + total + 161) % 211
										end
									end
								end
							else
								if key < 165 then
									if key < 163 then
										total = (total + 162 * step) % 10007
										cells[19] += 1
									else
										if key < 164 then
											if total % 2 == 0 then
												total += 163
											else
												total -= 7
											end
										else
											total = (total * 3 + cells[21]) % 10007
										end
									end
								else
									if key < 166 then
										cells[10] = (cells[10] + total + 165) % 211
									else
										if key < 167 then
											total = (total + 166 * step) % 10007
											cells[23] += 1
										else
											if total % 2 == 0 then
												total += 167
											else
												total -= 11
											end
										end
									end
								end
							end
						end
					else
						if key < 180 then
							if key < 174 then
								if key < 171 then
									if key < 169 then
										total = (total * 3 + cells[25]) % 10007
									else
										if key < 170 then
											cells[30] = (cells[30] + total + 169) % 211
										else
											total = (total + 170 * step) % 10007
											cells[27] += 1
										end
									end
								else
									if key < 172 then
										if total % 2 == 0 then
											total += 171
										else
											total -= 2
										end
									else
										if key < 173 then
											total = (total * 3 + cells[29]) % 10007
										else
											cells[2] = (cells[2] + total + 173) % 211
										end
									end
								end
							else
								if key < 177 then
									if key < 175 then
										total = (total + 174 * step) % 10007
										cells[31] += 1
									else
										if key < 176 then
											if total % 2 == 0 then
												total += 175
											else
												total -= 6
											end
										else
											total = (total * 3 + cells[33]) % 10007
										end
									end
								else
									if key < 178 then
										cells[22] = (cells[22] + total + 177) % 211
									else
										if key < 179 then
											total = (total + 178 * step) % 10007
											cells[35] += 1
										else
											if total % 2 == 0 then
												total += 179
											else
												total -= 10
											end
										end
									end
								end
							end
						else
							if key < 186 then
								if key < 183 then
									if key < 181 then
										total = (total * 3 + cells[37]) % 10007
									else
										if key < 182 then
											cells[42] = (cells[42] + total + 181) % 211
										else
											total = (total + 182 * step) % 10007
											cells[39] += 1
										end
									end
								else
									if key < 184 then
										if total % 2 == 0 then
											total += 183
										else
											total -= 1
										end
									else
										if key < 185 then
											total = (total * 3 + cells[41]) % 10007
										else
											cells[14] = (cells[14] + total + 185) % 211
										end
									end
								end
							else
								if key < 189 then
									if key < 187 then
										total = (total + 186 * step) % 10007
										cells[43] += 1
									else
										if key < 188 then
											if total % 2 == 0 then
												total += 187
											else
												total -= 5
											end
										else
											total = (total * 3 + cells[45]) % 10007
										end
									end
								else
									if key < 190 then
										cells[34] = (cells[34] + total + 189) % 211
									else
										if key < 191 then
											total = (total + 190 * step) % 10007
											cells[47] += 1
										else
											if total % 2 == 0 then
												total += 191
											else
												total -= 9
											end
										end
									end
								end
							end
						end
					end
				end
			end
		end
	until step >= rounds
	return total
end
local codes = {}
for i = 1, 90 do
	codes[i] = (i * i + 5) % 251
end
print(run(codes, 3), cells[1], cells[24], cells[48])
