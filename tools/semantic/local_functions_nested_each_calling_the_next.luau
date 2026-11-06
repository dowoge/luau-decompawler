-- local functions nested deep, each called once by the one around it: -O2
-- copies each into its parent, and every copy makes the closures it held
local base = tonumber("0")
local function step1() local v1 = base + 1
local function step2() local v2 = (v1 or base) + 2
local function step3() local v3 = (v2 or base) + 3
local function step4() local v4 = (v3 or base) + 4
local function step5() local v5 = (v4 or base) + 5
local function step6() local v6 = (v5 or base) + 6
local function step7() local v7 = (v6 or base) + 7
local function step8() local v8 = (v7 or base) + 8
local function step9() local v9 = (v8 or base) + 9
local function step10() local v10 = (v9 or base) + 10
local function step11() local v11 = (v10 or base) + 11
local function step12() local v12 = (v11 or base) + 12
local function step13() local v13 = (v12 or base) + 13
local function step14() local v14 = (v13 or base) + 14
local function step15() local v15 = (v14 or base) + 15
local function step16() local v16 = (v15 or base) + 16
local function step17() local v17 = (v16 or base) + 17
local function step18() local v18 = (v17 or base) + 18
local function step19() local v19 = (v18 or base) + 19
local function step20() local v20 = (v19 or base) + 20
local function step21() local v21 = (v20 or base) + 21
local function step22() local v22 = (v21 or base) + 22
local function step23() local v23 = (v22 or base) + 23
return v23
end
return step23() end
return step22() end
return step21() end
return step20() end
return step19() end
return step18() end
return step17() end
return step16() end
return step15() end
return step14() end
return step13() end
return step12() end
return step11() end
return step10() end
return step9() end
return step8() end
return step7() end
return step6() end
return step5() end
return step4() end
return step3() end
return step2() end
print(step1())
