-- An interpolated string is a literal: to call a method on it or index it
-- the source puts it in parentheses, and so must the decompiled text
local item = { Name = tostring("Sword"), Skin = "Gold", id = tostring(7) }
local key = (`{item.Name}:{item.Skin}:{item.id}`):lower()
print(key, (`{item.Name}`):gsub("S", "s"))

local function shout(x)
  return (`<{x}>`):upper(), (`{x}{x}`):len()
end
print(shout("ab"))
print(#(`{item.id}`):rep(3), (`{item.Skin}!`):sub(1, 2) .. `{item.id}`)
