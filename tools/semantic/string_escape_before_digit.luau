-- A byte written as a decimal escape, then a digit: `\7` and `1` must not
-- read back as `\71` (the letter G). Every such byte keeps its three digits.
local bell = "\0071"
local mixed = "a\0012\0273x\2009"
print(#bell, string.byte(bell, 1, -1))
print(#mixed, string.byte(mixed, 1, -1))
local t = { "\0310", "\1278" }
print(#t[1], string.byte(t[1], 1, -1), #t[2], string.byte(t[2], 1, -1))
