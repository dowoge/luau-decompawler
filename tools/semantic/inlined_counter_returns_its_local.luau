-- `count += 1 return count` copied into a call's arguments: the copy adds
-- into the local itself and moves it into its argument's place. That move
-- is the return's, not a store into the local.
local count = 0
local function bump()
    count += 1
    return count
end
local v = count
print(v, bump(), count, v + count)
for i = 1, 3 do
    print(i, bump(), count)
end
local rows = {}
for _, name in {"a", "b"} do
    table.insert(rows, name .. ":" .. tostring(bump()))
end
print(table.concat(rows, " "), count)
