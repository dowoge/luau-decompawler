-- 120 `do` blocks, each with two locals a stored closure captures. Kept as
-- blocks the locals come and go; in one flat scope they pass the 200-local limit.
local seed = tonumber("2")
local depots = { 4, 0, 7, 1, 3 }
local shop = {}
do
	local price = seed * 1 + 0
	local stock = depots[1]
	shop[1] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 2 + 1
	local stock = depots[2]
	shop[2] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 3 + 2
	local stock = depots[3]
	shop[3] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 4 + 3
	local stock = depots[4]
	shop[4] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 5 + 4
	local stock = depots[5]
	shop[5] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 6 + 5
	local stock = depots[1]
	shop[6] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 7 + 6
	local stock = depots[2]
	shop[7] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 8 + 7
	local stock = depots[3]
	shop[8] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 9 + 8
	local stock = depots[4]
	shop[9] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 1 + 9
	local stock = depots[5]
	shop[10] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 2 + 10
	local stock = depots[1]
	shop[11] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 3 + 11
	local stock = depots[2]
	shop[12] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 4 + 12
	local stock = depots[3]
	shop[13] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 5 + 13
	local stock = depots[4]
	shop[14] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 6 + 14
	local stock = depots[5]
	shop[15] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 7 + 15
	local stock = depots[1]
	shop[16] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 8 + 16
	local stock = depots[2]
	shop[17] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 9 + 17
	local stock = depots[3]
	shop[18] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 1 + 18
	local stock = depots[4]
	shop[19] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 2 + 19
	local stock = depots[5]
	shop[20] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 3 + 20
	local stock = depots[1]
	shop[21] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 4 + 21
	local stock = depots[2]
	shop[22] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 5 + 22
	local stock = depots[3]
	shop[23] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 6 + 23
	local stock = depots[4]
	shop[24] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 7 + 24
	local stock = depots[5]
	shop[25] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 8 + 25
	local stock = depots[1]
	shop[26] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 9 + 26
	local stock = depots[2]
	shop[27] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 1 + 27
	local stock = depots[3]
	shop[28] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 2 + 28
	local stock = depots[4]
	shop[29] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 3 + 29
	local stock = depots[5]
	shop[30] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 4 + 30
	local stock = depots[1]
	shop[31] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 5 + 31
	local stock = depots[2]
	shop[32] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 6 + 32
	local stock = depots[3]
	shop[33] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 7 + 33
	local stock = depots[4]
	shop[34] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 8 + 34
	local stock = depots[5]
	shop[35] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 9 + 35
	local stock = depots[1]
	shop[36] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 1 + 36
	local stock = depots[2]
	shop[37] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 2 + 37
	local stock = depots[3]
	shop[38] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 3 + 38
	local stock = depots[4]
	shop[39] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 4 + 39
	local stock = depots[5]
	shop[40] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 5 + 40
	local stock = depots[1]
	shop[41] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 6 + 41
	local stock = depots[2]
	shop[42] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 7 + 42
	local stock = depots[3]
	shop[43] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 8 + 43
	local stock = depots[4]
	shop[44] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 9 + 44
	local stock = depots[5]
	shop[45] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 1 + 45
	local stock = depots[1]
	shop[46] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 2 + 46
	local stock = depots[2]
	shop[47] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 3 + 47
	local stock = depots[3]
	shop[48] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 4 + 48
	local stock = depots[4]
	shop[49] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 5 + 49
	local stock = depots[5]
	shop[50] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 6 + 50
	local stock = depots[1]
	shop[51] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 7 + 51
	local stock = depots[2]
	shop[52] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 8 + 52
	local stock = depots[3]
	shop[53] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 9 + 53
	local stock = depots[4]
	shop[54] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 1 + 54
	local stock = depots[5]
	shop[55] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 2 + 55
	local stock = depots[1]
	shop[56] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 3 + 56
	local stock = depots[2]
	shop[57] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 4 + 57
	local stock = depots[3]
	shop[58] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 5 + 58
	local stock = depots[4]
	shop[59] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 6 + 59
	local stock = depots[5]
	shop[60] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 7 + 60
	local stock = depots[1]
	shop[61] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 8 + 61
	local stock = depots[2]
	shop[62] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 9 + 62
	local stock = depots[3]
	shop[63] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 1 + 63
	local stock = depots[4]
	shop[64] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 2 + 64
	local stock = depots[5]
	shop[65] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 3 + 65
	local stock = depots[1]
	shop[66] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 4 + 66
	local stock = depots[2]
	shop[67] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 5 + 67
	local stock = depots[3]
	shop[68] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 6 + 68
	local stock = depots[4]
	shop[69] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 7 + 69
	local stock = depots[5]
	shop[70] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 8 + 70
	local stock = depots[1]
	shop[71] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 9 + 71
	local stock = depots[2]
	shop[72] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 1 + 72
	local stock = depots[3]
	shop[73] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 2 + 73
	local stock = depots[4]
	shop[74] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 3 + 74
	local stock = depots[5]
	shop[75] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 4 + 75
	local stock = depots[1]
	shop[76] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 5 + 76
	local stock = depots[2]
	shop[77] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 6 + 77
	local stock = depots[3]
	shop[78] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 7 + 78
	local stock = depots[4]
	shop[79] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 8 + 79
	local stock = depots[5]
	shop[80] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 9 + 80
	local stock = depots[1]
	shop[81] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 1 + 81
	local stock = depots[2]
	shop[82] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 2 + 82
	local stock = depots[3]
	shop[83] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 3 + 83
	local stock = depots[4]
	shop[84] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 4 + 84
	local stock = depots[5]
	shop[85] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 5 + 85
	local stock = depots[1]
	shop[86] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 6 + 86
	local stock = depots[2]
	shop[87] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 7 + 87
	local stock = depots[3]
	shop[88] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 8 + 88
	local stock = depots[4]
	shop[89] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 9 + 89
	local stock = depots[5]
	shop[90] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 1 + 90
	local stock = depots[1]
	shop[91] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 2 + 91
	local stock = depots[2]
	shop[92] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 3 + 92
	local stock = depots[3]
	shop[93] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 4 + 93
	local stock = depots[4]
	shop[94] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 5 + 94
	local stock = depots[5]
	shop[95] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 6 + 95
	local stock = depots[1]
	shop[96] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 7 + 96
	local stock = depots[2]
	shop[97] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 8 + 97
	local stock = depots[3]
	shop[98] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 9 + 98
	local stock = depots[4]
	shop[99] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 1 + 99
	local stock = depots[5]
	shop[100] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 2 + 100
	local stock = depots[1]
	shop[101] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 3 + 101
	local stock = depots[2]
	shop[102] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 4 + 102
	local stock = depots[3]
	shop[103] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 5 + 103
	local stock = depots[4]
	shop[104] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 6 + 104
	local stock = depots[5]
	shop[105] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 7 + 105
	local stock = depots[1]
	shop[106] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 8 + 106
	local stock = depots[2]
	shop[107] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 9 + 107
	local stock = depots[3]
	shop[108] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 1 + 108
	local stock = depots[4]
	shop[109] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 2 + 109
	local stock = depots[5]
	shop[110] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 3 + 110
	local stock = depots[1]
	shop[111] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 4 + 111
	local stock = depots[2]
	shop[112] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 5 + 112
	local stock = depots[3]
	shop[113] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 6 + 113
	local stock = depots[4]
	shop[114] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 7 + 114
	local stock = depots[5]
	shop[115] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 8 + 115
	local stock = depots[1]
	shop[116] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 9 + 116
	local stock = depots[2]
	shop[117] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 1 + 117
	local stock = depots[3]
	shop[118] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 2 + 118
	local stock = depots[4]
	shop[119] = function(qty)
		return price * qty - stock
	end
end
do
	local price = seed * 3 + 119
	local stock = depots[5]
	shop[120] = function(qty)
		return price * qty - stock
	end
end
local sum = 0
for i, buy in shop do
	sum += buy(i % 3)
end
print(#shop, sum, shop[1](10), shop[120](1))
