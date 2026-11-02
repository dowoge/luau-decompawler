-- the shape of real game scripts
local Config = {MaxHealth = 100, Regen = 2, Speed = {Walk = 16, Run = 24}, Names = {"Alpha", "Beta"}}
local Players = {}
local Connections = {}
local Events = {}
local function now() return #Connections end
local function fire(name, ...)
    local list = Events[name]
    if not list then return end
    for _, fn in ipairs(list) do
        fn(...)
    end
end
local function on(name, fn)
    Events[name] = Events[name] or {}
    table.insert(Events[name], fn)
    return function()
        local list = Events[name]
        for i, f in ipairs(list) do
            if f == fn then
                table.remove(list, i)
                break
            end
        end
    end
end
local Player = {}
Player.__index = Player
function Player.new(id, name)
    local self = setmetatable({}, Player)
    self.Id = id
    self.Name = name or Config.Names[(id - 1) % #Config.Names + 1]
    self.Health = Config.MaxHealth
    self.Inventory = {}
    self.Alive = true
    self.Team = nil
    Players[id] = self
    fire("joined", self)
    return self
end
function Player:TakeDamage(amount, source)
    if not self.Alive then return false end
    local before = self.Health
    self.Health = math.max(0, self.Health - amount)
    if self.Health == 0 then
        self.Alive = false
        fire("died", self, source)
    elseif before > 50 and self.Health <= 50 then
        fire("hurt", self)
    end
    return true
end
function Player:Heal(amount)
    if self.Alive and self.Health < Config.MaxHealth then
        self.Health = math.min(Config.MaxHealth, self.Health + (amount or Config.Regen))
    end
end
function Player:AddItem(name, count)
    count = count or 1
    local slot = self.Inventory[name]
    if slot then
        slot.Count += count
    else
        self.Inventory[name] = {Count = count, Added = now()}
    end
    return self.Inventory[name].Count
end
function Player:RemoveItem(name, count)
    local slot = self.Inventory[name]
    if not slot or slot.Count < (count or 1) then
        return false, "not enough " .. name
    end
    slot.Count -= count or 1
    if slot.Count <= 0 then
        self.Inventory[name] = nil
    end
    return true
end
function Player:Speed(running)
    local s = Config.Speed
    return if running and self.Health > 20 then s.Run else s.Walk
end
local function sortedPlayers(by)
    local out = {}
    for _, p in pairs(Players) do
        table.insert(out, p)
    end
    table.sort(out, function(a, b)
        if a[by] == b[by] then
            return a.Id < b.Id
        end
        return a[by] > b[by]
    end)
    return out
end
local Round = {State = "idle", Timer = 0, Winners = {}}
function Round.Step(dt)
    if Round.State == "idle" then
        if #sortedPlayers("Id") >= 2 then
            Round.State = "countdown"
            Round.Timer = 3
        end
    elseif Round.State == "countdown" then
        Round.Timer -= dt
        if Round.Timer <= 0 then
            Round.State = "playing"
            Round.Timer = 10
            fire("start")
        end
    elseif Round.State == "playing" then
        Round.Timer -= dt
        local alive = 0
        local last
        for _, p in pairs(Players) do
            if p.Alive then
                alive += 1
                last = p
            end
        end
        if alive <= 1 or Round.Timer <= 0 then
            Round.State = "over"
            if last and alive == 1 then
                table.insert(Round.Winners, last.Name)
            end
            fire("over", last)
        end
    end
    return Round.State
end
local function serialize(v, depth)
    depth = depth or 0
    local t = type(v)
    if t == "table" then
        if depth > 3 then return "..." end
        local keys = {}
        for k in pairs(v) do
            table.insert(keys, tostring(k))
        end
        table.sort(keys)
        local parts = {}
        for _, k in ipairs(keys) do
            local item = v[k]
            if item == nil then item = v[tonumber(k)] end
            if type(item) ~= "function" then
                table.insert(parts, k .. "=" .. serialize(item, depth + 1))
            end
        end
        return "{" .. table.concat(parts, ",") .. "}"
    elseif t == "string" then
        return string.format("%q", v)
    end
    return tostring(v)
end
local log = {}
local off = on("joined", function(p) table.insert(log, "join " .. p.Name) end)
on("died", function(p, s) table.insert(log, p.Name .. " killed by " .. tostring(s and s.Name)) end)
on("hurt", function(p) table.insert(log, p.Name .. " hurt") end)
on("start", function() table.insert(log, "start") end)
on("over", function(w) table.insert(log, "over " .. (w and w.Name or "nobody")) end)
local a, b = Player.new(1), Player.new(2, "Zed")
off()
Player.new(3)
print(a.Name, b.Name, Round.Step(1), Round.Step(1), Round.Step(3), Round.Step(1))
a:AddItem("sword") a:AddItem("sword", 2) print(a:RemoveItem("sword", 5)) print(a:RemoveItem("sword")) print(a:RemoveItem("sword", 2), a.Inventory.sword)
print(a:TakeDamage(60, b), a:Speed(true), a:TakeDamage(30), a:Speed(true)) a:Heal() a:Heal(3) print(a.Health)
print(a:TakeDamage(50, b), a:TakeDamage(1), b:TakeDamage(200), Round.Step(1), table.concat(Round.Winners, ","))
print(serialize({a = 1, b = "x", c = {1, 2, {d = true, e = {f = {g = 1}}}}, h = print}))
for _, p in ipairs(sortedPlayers("Health")) do print(p.Id, p.Health, p.Alive) end
print(table.concat(log, ";"))
