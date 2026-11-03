-- A function whose parameters leave a handful of the 200 locals Luau allows:
-- tables read once and conditional values among the arguments of calls stay
-- inside those calls. Each made a local of its own, the text would not fit.
local seen = {}
local function rec(...)
    local n = select("#", ...)
    seen[#seen + 1] = n
    return n
end
local function num(x)
    return type(x) == "number" and x or #tostring(x)
end
local function sum(t)
    local s = 0
    for _, v in t do
        s += num(v)
    end
    return s
end
local function wide(a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, a23, a24, a25, a26, a27, a28, a29, a30, a31, a32, a33, a34, a35, a36, a37, a38, a39, a40, a41, a42, a43, a44, a45, a46, a47, a48, a49, a50, a51, a52, a53, a54, a55, a56, a57, a58, a59, a60, a61, a62, a63, a64, a65, a66, a67, a68, a69, a70, a71, a72, a73, a74, a75, a76, a77, a78, a79, a80, a81, a82, a83, a84, a85, a86, a87, a88, a89, a90, a91, a92, a93, a94, a95, a96, a97, a98, a99, a100, a101, a102, a103, a104, a105, a106, a107, a108, a109, a110, a111, a112, a113, a114, a115, a116, a117, a118, a119, a120, a121, a122, a123, a124, a125, a126, a127, a128, a129, a130, a131, a132, a133, a134, a135, a136, a137, a138, a139, a140, a141, a142, a143, a144, a145, a146, a147, a148, a149, a150, a151, a152, a153, a154, a155, a156, a157, a158, a159, a160, a161, a162, a163, a164, a165, a166, a167, a168, a169, a170, a171, a172, a173, a174, a175, a176, a177, a178, a179, a180, a181, a182, a183, a184, a185)
    for round = 1, 2 do
        rec(sum({ a0, a5 * 2, num(a11) }), if a0 > a5 then a11 else -a11, sum({ a1, a8, round }), a13 > 50 and a0 or a5, rec(a9, { k = a14 }), num(if round == 1 then a15 else a16))
        rec(sum({ a31, a22 * 2, num(a18) }), if a31 > a22 then a18 else -a18, sum({ a32, a25, round }), a20 > 50 and a31 or a22, rec(a40, { k = a31 }), num(if round == 1 then a22 else a23))
        rec(sum({ a62, a39 * 2, num(a25) }), if a62 > a39 then a25 else -a25, sum({ a63, a42, round }), a27 > 50 and a62 or a39, rec(a71, { k = a48 }), num(if round == 1 then a29 else a30))
        rec(sum({ a93, a56 * 2, num(a32) }), if a93 > a56 then a32 else -a32, sum({ a94, a59, round }), a34 > 50 and a93 or a56, rec(a102, { k = a65 }), num(if round == 1 then a36 else a37))
        rec(sum({ a124, a73 * 2, num(a39) }), if a124 > a73 then a39 else -a39, sum({ a125, a76, round }), a41 > 50 and a124 or a73, rec(a133, { k = a82 }), num(if round == 1 then a43 else a44))
        rec(sum({ a155, a90 * 2, num(a46) }), if a155 > a90 then a46 else -a46, sum({ a156, a93, round }), a48 > 50 and a155 or a90, rec(a164, { k = a99 }), num(if round == 1 then a50 else a51))
        local last = rec({ a185, a184 }, if round > 1 then a0 else a1)
        seen[#seen + 1] = last
    end
    return sum({ a3, a4, #seen })
end
local args = {}
for i = 1, 186 do
    args[i] = i * 3 % 97
end
print(wide(table.unpack(args)))
print(#seen, table.concat(seen, ","))
