local Animal = {}
Animal.__index = Animal
function Animal.new(name, sound)
    local self = setmetatable({}, Animal)
    self.name = name
    self.sound = sound
    self.count = 0
    return self
end
function Animal:speak()
    self.count += 1
    return self.name .. " says " .. self.sound
end
function Animal:rename(name)
    local old = self.name
    self.name = name
    return old
end
local Dog = setmetatable({}, {__index = Animal})
Dog.__index = Dog
function Dog.new(name)
    local self = Animal.new(name, "woof")
    return setmetatable(self, Dog)
end
function Dog:fetch(item)
    if item == nil then
        return self.name .. " has nothing"
    end
    self.last = item
    return self.name .. " fetched " .. item
end
function Dog:speak()
    local base = Animal.speak(self)
    return base .. "!"
end
local Stack = {}
Stack.__index = Stack
function Stack.new() return setmetatable({items = {}, size = 0}, Stack) end
function Stack:push(v) self.size += 1 self.items[self.size] = v return self end
function Stack:pop()
    if self.size == 0 then return nil end
    local v = self.items[self.size]
    self.items[self.size] = nil
    self.size -= 1
    return v
end
function Stack:peek() return self.items[self.size] end
function Stack.__len(s) return s.size end
function Stack.__tostring(s) return "Stack(" .. s.size .. ")" end
function Stack.__eq(a, b) return a.size == b.size end
function Stack.__add(a, b)
    local r = Stack.new()
    for i = 1, a.size do r:push(a.items[i]) end
    for i = 1, b.size do r:push(b.items[i]) end
    return r
end
local Vec = {}
Vec.__index = Vec
local function vec(x, y) return setmetatable({x = x, y = y}, Vec) end
function Vec.__add(a, b) return vec(a.x + b.x, a.y + b.y) end
function Vec.__mul(a, b)
    if type(b) == "number" then return vec(a.x * b, a.y * b) end
    return a.x * b.x + a.y * b.y
end
function Vec:length() return math.sqrt(self.x * self.x + self.y * self.y) end
function Vec:unit() local l = self:length() if l == 0 then return vec(0, 0) end return vec(self.x / l, self.y / l) end
local Signal = {}
Signal.__index = Signal
function Signal.new() return setmetatable({handlers = {}, nextId = 1}, Signal) end
function Signal:Connect(fn)
    local id = self.nextId
    self.nextId += 1
    self.handlers[id] = fn
    local conn = {}
    function conn.Disconnect() self.handlers[id] = nil end
    return conn
end
function Signal:Fire(...)
    local ids = {}
    for id in pairs(self.handlers) do table.insert(ids, id) end
    table.sort(ids)
    for _, id in ipairs(ids) do self.handlers[id](...) end
end
local a = Animal.new("cat", "meow") print(a:speak(), a.count, a:rename("tom"), a:speak())
local d = Dog.new("rex") print(d:speak(), d:fetch(), d:fetch("ball"), d.last)
local s = Stack.new() s:push(1):push(2) print(s:pop(), s:peek(), #s, tostring(s), s == Stack.new(), tostring(s + s), Stack.new():pop())
print((vec(3, 4) + vec(1, 1)).x, (vec(1, 2) * 2).y, vec(1, 2) * vec(3, 4), vec(3, 4):length(), vec(0, 0):unit().x)
local sig = Signal.new() local c = sig:Connect(function(x) print("a", x) end) sig:Connect(function(x) print("b", x) end) sig:Fire(1) c.Disconnect() sig:Fire(2)
