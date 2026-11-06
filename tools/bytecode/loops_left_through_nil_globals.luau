print(1)
local fn = function()
    print("This might not appear")
    while wait() do
    end
end

local fn2 = function()
    return "This will definitely appear"
end

local callback = fn2
while true do
    if callback ~= fn2 then break end
    callback = fn
    repeat
        if not _ then break end
    until true
    if not callback then continue end
end

callback()
print(2)
local n2 = 1
while true do
    if not (n2 <= 1) then break end
    n2 = 12345
    repeat
        if not __ then break end
    until true
    if not n2 then continue end
end

print("might be 1, might be", n2)
if n2 > 5 then
    print("running 5 < a")
end

local n = n2 / 5
print("Maybe 1/5, maybe", n)
if n % 1 == 0 then
    print("b%1 == 0")
end
