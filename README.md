# luau-decompiler

Usage:

```sh
git submodule update --init
cmake -S . -B build
cmake --build build --config Release --target luau_check luau_run
cargo build --release

target/release/luau_decompile script.wire
target/release/luau_decompile --upstream script.luauc
target/release/luau_decompile --dir dumps --out decompiled
cargo harness check --full
```

<table>
<tr><th>Original</th><th>Decompiled from -O2 bytecode</th></tr>
<tr>
<td>

```lua
local Players = game:GetService("Players")

Players.PlayerAdded:Connect(function(player)
    local leaderstats = Instance.new("Folder")
    leaderstats.Name = "leaderstats"
    leaderstats.Parent = player

    local coins = Instance.new("IntValue")
    coins.Name = "Coins"
    coins.Parent = leaderstats

    player.CharacterAdded:Connect(function(character)
        local humanoid = character:WaitForChild("Humanoid")
        humanoid.Died:Connect(function()
            coins.Value = math.max(coins.Value - 10, 0)
        end)
    end)
end)
```

</td>
<td>

```lua
local Players = game:GetService("Players")
Players.PlayerAdded:Connect(function(player)
    local leaderstats = Instance.new("Folder")
    leaderstats.Name = "leaderstats"
    leaderstats.Parent = player
    local coins = Instance.new("IntValue")
    coins.Name = "Coins"
    coins.Parent = leaderstats
    player.CharacterAdded:Connect(function(character)
        local humanoid = character:WaitForChild("Humanoid")
        humanoid.Died:Connect(function()
            coins.Value = math.max(coins.Value - 10, 0)
        end)
    end)
end)
```

</td>
</tr>
<tr>
<td>

```lua
local function lerp(a, b, t)
    return a + (b - a) * t
end

local function follow(state, dt)
    local alpha = math.min(dt * 8, 1)
    state.x = lerp(state.x, state.targetX, alpha)
    state.y = lerp(state.y, state.targetY, alpha)
end

return follow
```

</td>
<td>

```lua
local function lerp(x, targetX, n)
    return x + (targetX - x) * n
end

local function follow(t, n)
    local n2 = math.min(n * 8, 1)
    t.x = lerp(t.x, t.targetX, n2)
    t.y = lerp(t.y, t.targetY, n2)
end

return follow
```

</td>
</tr>
<tr>
<td>

```lua
local function summarize(items)
    local total, skipped = 0, 0
    for _, item in items do
        if not item.enabled then
            skipped += 1
            continue
        end
        total += item.price * (item.count or 1)
    end
    local label = if total > 100 then "big" else "small"
    return `{label}: {total} ({skipped} skipped)`
end

return summarize
```

</td>
<td>

```lua
local function summarize(list)
    local total = 0
    local count = 0
    for _, v in list do
        if not v.enabled then
            count += 1
        else
            total += v.price * (v.count or 1)
        end
    end
    return `{total > 100 and "big" or "small"}: {total} ({count} skipped)`
end

return summarize
```

</td>
</tr>
<tr>
<td>

```lua
local Stack = {}
Stack.__index = Stack

function Stack.new()
    return setmetatable({ items = {}, size = 0 }, Stack)
end

function Stack:push(value)
    self.size += 1
    self.items[self.size] = value
end

function Stack:pop()
    if self.size == 0 then
        return nil
    end
    local value = self.items[self.size]
    self.items[self.size] = nil
    self.size -= 1
    return value
end

return Stack
```

</td>
<td>

```lua
local Class = {}
Class.__index = Class

function Class.new()
    return setmetatable({items = {}, size = 0}, Class)
end

function Class:push(item)
    self.size += 1
    self.items[self.size] = item
end

function Class:pop()
    if self.size == 0 then
        return nil
    end
    local item = self.items[self.size]
    self.items[self.size] = nil
    self.size -= 1
    return item
end

return Class
```

</td>
</tr>
</table>
