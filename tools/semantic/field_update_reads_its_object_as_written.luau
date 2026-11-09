-- `a.b.k = a.b.k + v` reads `a.b` twice, `a.b.k += v` once: an __index that
-- counts its calls tells the two apart, so neither may become the other
local reads = 0
local inner = { Velocity = 1, Text = "a" }
local obj = setmetatable({}, {
  __index = function(_, _)
    reads += 1
    return inner
  end,
})

local function expanded(v)
  obj.Part.Velocity = obj.Part.Velocity + v
  obj.Part.Text = obj.Part.Text .. "b" .. v
end

local function compound(v)
  obj.Part.Velocity += v
  obj.Part.Text ..= "c"
end

expanded(2)
print(reads, inner.Velocity, inner.Text)
compound(3)
print(reads, inner.Velocity, inner.Text)

-- the same through a global, and one level down through a parameter
proxy = obj
local function viaGlobal(v, t)
  proxy.Part.Velocity = proxy.Part.Velocity * v
  proxy.Part.Velocity *= v
  t.Velocity = t.Velocity - 1
  t.Velocity -= 1
end
viaGlobal(2, inner)
print(reads, inner.Velocity)
