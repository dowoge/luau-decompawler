-- A long function where every branch may call the same upvalue: a value
-- defined at entry and read in hundreds of places.
local penalty = function(v)
	return v % 17 - 3
end
local function tally(a, b)
	local out = {}
	do
		local v = (a - 0) * b + 0
		if v < 0 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 1) * b + 1
		if v < 2 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 2) * b + 2
		if v < 4 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 3) * b + 3
		if v < 6 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 4) * b + 4
		if v < 8 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 5) * b + 0
		if v < 10 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 6) * b + 1
		if v < 12 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 7) * b + 2
		if v < 14 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 8) * b + 3
		if v < 16 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 9) * b + 4
		if v < 18 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 10) * b + 0
		if v < 20 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 11) * b + 1
		if v < 22 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 12) * b + 2
		if v < 24 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 13) * b + 3
		if v < 26 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 14) * b + 4
		if v < 28 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 15) * b + 0
		if v < 30 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 16) * b + 1
		if v < 32 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 17) * b + 2
		if v < 34 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 18) * b + 3
		if v < 36 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 19) * b + 4
		if v < 38 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 20) * b + 0
		if v < 40 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 21) * b + 1
		if v < 42 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 22) * b + 2
		if v < 44 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 23) * b + 3
		if v < 46 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 24) * b + 4
		if v < 48 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 25) * b + 0
		if v < 50 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 26) * b + 1
		if v < 52 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 27) * b + 2
		if v < 54 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 28) * b + 3
		if v < 56 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 29) * b + 4
		if v < 58 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 30) * b + 0
		if v < 60 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 31) * b + 1
		if v < 62 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 32) * b + 2
		if v < 64 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 33) * b + 3
		if v < 66 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 34) * b + 4
		if v < 68 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 35) * b + 0
		if v < 70 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 36) * b + 1
		if v < 72 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 37) * b + 2
		if v < 74 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 38) * b + 3
		if v < 76 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 39) * b + 4
		if v < 78 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 40) * b + 0
		if v < 80 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 41) * b + 1
		if v < 82 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 42) * b + 2
		if v < 84 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 43) * b + 3
		if v < 86 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 44) * b + 4
		if v < 88 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 45) * b + 0
		if v < 90 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 46) * b + 1
		if v < 92 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 47) * b + 2
		if v < 94 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 48) * b + 3
		if v < 96 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 49) * b + 4
		if v < 98 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 50) * b + 0
		if v < 100 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 51) * b + 1
		if v < 102 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 52) * b + 2
		if v < 104 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 53) * b + 3
		if v < 106 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 54) * b + 4
		if v < 108 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 55) * b + 0
		if v < 110 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 56) * b + 1
		if v < 112 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 57) * b + 2
		if v < 114 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 58) * b + 3
		if v < 116 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 59) * b + 4
		if v < 118 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 60) * b + 0
		if v < 120 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 61) * b + 1
		if v < 122 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 62) * b + 2
		if v < 124 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 63) * b + 3
		if v < 126 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 64) * b + 4
		if v < 128 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 65) * b + 0
		if v < 130 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 66) * b + 1
		if v < 132 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 67) * b + 2
		if v < 134 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 68) * b + 3
		if v < 136 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 69) * b + 4
		if v < 138 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 70) * b + 0
		if v < 140 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 71) * b + 1
		if v < 142 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 72) * b + 2
		if v < 144 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 73) * b + 3
		if v < 146 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 74) * b + 4
		if v < 148 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 75) * b + 0
		if v < 150 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 76) * b + 1
		if v < 152 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 77) * b + 2
		if v < 154 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 78) * b + 3
		if v < 156 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 79) * b + 4
		if v < 158 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 80) * b + 0
		if v < 160 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 81) * b + 1
		if v < 162 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 82) * b + 2
		if v < 164 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 83) * b + 3
		if v < 166 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 84) * b + 4
		if v < 168 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 85) * b + 0
		if v < 170 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 86) * b + 1
		if v < 172 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 87) * b + 2
		if v < 174 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 88) * b + 3
		if v < 176 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 89) * b + 4
		if v < 178 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 90) * b + 0
		if v < 180 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 91) * b + 1
		if v < 182 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 92) * b + 2
		if v < 184 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 93) * b + 3
		if v < 186 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 94) * b + 4
		if v < 188 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 95) * b + 0
		if v < 190 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 96) * b + 1
		if v < 192 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 97) * b + 2
		if v < 194 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 98) * b + 3
		if v < 196 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 99) * b + 4
		if v < 198 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 100) * b + 0
		if v < 200 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 101) * b + 1
		if v < 202 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 102) * b + 2
		if v < 204 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 103) * b + 3
		if v < 206 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 104) * b + 4
		if v < 208 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 105) * b + 0
		if v < 210 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 106) * b + 1
		if v < 212 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 107) * b + 2
		if v < 214 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 108) * b + 3
		if v < 216 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 109) * b + 4
		if v < 218 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 110) * b + 0
		if v < 220 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 111) * b + 1
		if v < 222 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 112) * b + 2
		if v < 224 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 113) * b + 3
		if v < 226 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 114) * b + 4
		if v < 228 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 115) * b + 0
		if v < 230 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 116) * b + 1
		if v < 232 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 117) * b + 2
		if v < 234 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 118) * b + 3
		if v < 236 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 119) * b + 4
		if v < 238 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 120) * b + 0
		if v < 240 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 121) * b + 1
		if v < 242 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 122) * b + 2
		if v < 244 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 123) * b + 3
		if v < 246 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 124) * b + 4
		if v < 248 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 125) * b + 0
		if v < 250 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 126) * b + 1
		if v < 252 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 127) * b + 2
		if v < 254 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 128) * b + 3
		if v < 256 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 129) * b + 4
		if v < 258 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 130) * b + 0
		if v < 260 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 131) * b + 1
		if v < 262 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 132) * b + 2
		if v < 264 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 133) * b + 3
		if v < 266 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 134) * b + 4
		if v < 268 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 135) * b + 0
		if v < 270 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 136) * b + 1
		if v < 272 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 137) * b + 2
		if v < 274 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 138) * b + 3
		if v < 276 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 139) * b + 4
		if v < 278 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 140) * b + 0
		if v < 280 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 141) * b + 1
		if v < 282 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 142) * b + 2
		if v < 284 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 143) * b + 3
		if v < 286 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 144) * b + 4
		if v < 288 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 145) * b + 0
		if v < 290 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 146) * b + 1
		if v < 292 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 147) * b + 2
		if v < 294 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 148) * b + 3
		if v < 296 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 149) * b + 4
		if v < 298 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 150) * b + 0
		if v < 300 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 151) * b + 1
		if v < 302 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 152) * b + 2
		if v < 304 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 153) * b + 3
		if v < 306 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 154) * b + 4
		if v < 308 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 155) * b + 0
		if v < 310 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 156) * b + 1
		if v < 312 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 157) * b + 2
		if v < 314 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 158) * b + 3
		if v < 316 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 159) * b + 4
		if v < 318 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 160) * b + 0
		if v < 320 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 161) * b + 1
		if v < 322 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 162) * b + 2
		if v < 324 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 163) * b + 3
		if v < 326 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 164) * b + 4
		if v < 328 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 165) * b + 0
		if v < 330 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 166) * b + 1
		if v < 332 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 167) * b + 2
		if v < 334 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 168) * b + 3
		if v < 336 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 169) * b + 4
		if v < 338 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 170) * b + 0
		if v < 340 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 171) * b + 1
		if v < 342 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 172) * b + 2
		if v < 344 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 173) * b + 3
		if v < 346 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 174) * b + 4
		if v < 348 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 175) * b + 0
		if v < 350 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 176) * b + 1
		if v < 352 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 177) * b + 2
		if v < 354 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 178) * b + 3
		if v < 356 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 179) * b + 4
		if v < 358 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 180) * b + 0
		if v < 360 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 181) * b + 1
		if v < 362 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 182) * b + 2
		if v < 364 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 183) * b + 3
		if v < 366 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 184) * b + 4
		if v < 368 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 185) * b + 0
		if v < 370 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 186) * b + 1
		if v < 372 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 187) * b + 2
		if v < 374 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 188) * b + 3
		if v < 376 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 189) * b + 4
		if v < 378 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 190) * b + 0
		if v < 380 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 191) * b + 1
		if v < 382 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 192) * b + 2
		if v < 384 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 193) * b + 3
		if v < 386 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 194) * b + 4
		if v < 388 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 195) * b + 0
		if v < 390 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 196) * b + 1
		if v < 392 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 197) * b + 2
		if v < 394 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 198) * b + 3
		if v < 396 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 199) * b + 4
		if v < 398 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 200) * b + 0
		if v < 400 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 201) * b + 1
		if v < 402 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 202) * b + 2
		if v < 404 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 203) * b + 3
		if v < 406 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 204) * b + 4
		if v < 408 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 205) * b + 0
		if v < 410 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 206) * b + 1
		if v < 412 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 207) * b + 2
		if v < 414 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 208) * b + 3
		if v < 416 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 209) * b + 4
		if v < 418 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 210) * b + 0
		if v < 420 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 211) * b + 1
		if v < 422 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 212) * b + 2
		if v < 424 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 213) * b + 3
		if v < 426 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 214) * b + 4
		if v < 428 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 215) * b + 0
		if v < 430 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 216) * b + 1
		if v < 432 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 217) * b + 2
		if v < 434 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 218) * b + 3
		if v < 436 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 219) * b + 4
		if v < 438 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 220) * b + 0
		if v < 440 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 221) * b + 1
		if v < 442 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 222) * b + 2
		if v < 444 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 223) * b + 3
		if v < 446 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 224) * b + 4
		if v < 448 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 225) * b + 0
		if v < 450 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 226) * b + 1
		if v < 452 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 227) * b + 2
		if v < 454 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 228) * b + 3
		if v < 456 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 229) * b + 4
		if v < 458 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 230) * b + 0
		if v < 460 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 231) * b + 1
		if v < 462 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 232) * b + 2
		if v < 464 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 233) * b + 3
		if v < 466 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 234) * b + 4
		if v < 468 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 235) * b + 0
		if v < 470 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 236) * b + 1
		if v < 472 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 237) * b + 2
		if v < 474 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 238) * b + 3
		if v < 476 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 239) * b + 4
		if v < 478 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 240) * b + 0
		if v < 480 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 241) * b + 1
		if v < 482 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 242) * b + 2
		if v < 484 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 243) * b + 3
		if v < 486 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 244) * b + 4
		if v < 488 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 245) * b + 0
		if v < 490 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 246) * b + 1
		if v < 492 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 247) * b + 2
		if v < 494 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 248) * b + 3
		if v < 496 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 249) * b + 4
		if v < 498 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 250) * b + 0
		if v < 500 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 251) * b + 1
		if v < 502 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 252) * b + 2
		if v < 504 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 253) * b + 3
		if v < 506 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 254) * b + 4
		if v < 508 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 255) * b + 0
		if v < 510 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 256) * b + 1
		if v < 512 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 257) * b + 2
		if v < 514 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 258) * b + 3
		if v < 516 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 259) * b + 4
		if v < 518 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 260) * b + 0
		if v < 520 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 261) * b + 1
		if v < 522 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 262) * b + 2
		if v < 524 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 263) * b + 3
		if v < 526 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 264) * b + 4
		if v < 528 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 265) * b + 0
		if v < 530 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 266) * b + 1
		if v < 532 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 267) * b + 2
		if v < 534 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 268) * b + 3
		if v < 536 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 269) * b + 4
		if v < 538 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 270) * b + 0
		if v < 540 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 271) * b + 1
		if v < 542 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 272) * b + 2
		if v < 544 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 273) * b + 3
		if v < 546 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 274) * b + 4
		if v < 548 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 275) * b + 0
		if v < 550 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 276) * b + 1
		if v < 552 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 277) * b + 2
		if v < 554 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 278) * b + 3
		if v < 556 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 279) * b + 4
		if v < 558 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 280) * b + 0
		if v < 560 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 281) * b + 1
		if v < 562 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 282) * b + 2
		if v < 564 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 283) * b + 3
		if v < 566 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 284) * b + 4
		if v < 568 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 285) * b + 0
		if v < 570 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 286) * b + 1
		if v < 572 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 287) * b + 2
		if v < 574 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 288) * b + 3
		if v < 576 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 289) * b + 4
		if v < 578 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 290) * b + 0
		if v < 580 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 291) * b + 1
		if v < 582 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 292) * b + 2
		if v < 584 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 293) * b + 3
		if v < 586 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 294) * b + 4
		if v < 588 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 295) * b + 0
		if v < 590 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 296) * b + 1
		if v < 592 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 297) * b + 2
		if v < 594 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 298) * b + 3
		if v < 596 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 299) * b + 4
		if v < 598 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 300) * b + 0
		if v < 600 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 301) * b + 1
		if v < 602 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 302) * b + 2
		if v < 604 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 303) * b + 3
		if v < 606 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 304) * b + 4
		if v < 608 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 305) * b + 0
		if v < 610 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 306) * b + 1
		if v < 612 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 307) * b + 2
		if v < 614 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 308) * b + 3
		if v < 616 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 309) * b + 4
		if v < 618 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 310) * b + 0
		if v < 620 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 311) * b + 1
		if v < 622 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 312) * b + 2
		if v < 624 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 313) * b + 3
		if v < 626 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 314) * b + 4
		if v < 628 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 315) * b + 0
		if v < 630 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 316) * b + 1
		if v < 632 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 317) * b + 2
		if v < 634 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 318) * b + 3
		if v < 636 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	do
		local v = (a - 319) * b + 4
		if v < 638 then
			out[#out + 1] = penalty(v)
		else
			out[#out + 1] = v
		end
	end
	return out
end
local r = tally(150, 3)
local sum = 0
for _, v in r do
	sum += v
end
print(#r, sum, r[1], r[160], r[320])
