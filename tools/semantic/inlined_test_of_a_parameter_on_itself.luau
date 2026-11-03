-- A helper testing a parameter against itself (`p > p`), called with a
-- number: the parameter is a constant to the compiler, which folds the test
-- out of the copy. The copy is still the helper's.
local function f(p, q)
    if string.len(string.rep("bc", 0)) > p then
        return q
    end
    if p > p then
        return "never"
    end
    print(q, p, p)
    return true
end
print(f(1, "z"), f(-1, "y"))
for i = 1, 2 do
    print(f(1, "z"), f(i, i), f(-2, i))
end
