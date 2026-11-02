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
    local folder = Instance.new("Folder")
    folder.Name = "leaderstats"
    folder.Parent = player
    local intValue = Instance.new("IntValue")
    intValue.Name = "Coins"
    intValue.Parent = folder
    player.CharacterAdded:Connect(function(character)
        local Humanoid = character:WaitForChild("Humanoid")
        Humanoid.Died:Connect(function()
            intValue.Value = math.max(intValue.Value - 10, 0)
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
local function lerp(x, targetX, arg3_1)
    return x + (targetX - x) * arg3_1
end

local function follow(arg1_1, arg2_1)
    local v2_1 = math.min(arg2_1 * 8, 1)
    arg1_1.x = lerp(arg1_1.x, arg1_1.targetX, v2_1)
    arg1_1.y = lerp(arg1_1.y, arg1_1.targetY, v2_1)
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
local function summarize(arg1_1)
    local v1_1 = 0
    local v2_1 = 0
    for _, v7_1 in arg1_1 do
        if not v7_1.enabled then
            v2_1 += 1
        else
            v1_1 += v7_1.price * (v7_1.count or 1)
        end
    end
    return (`{v1_1 > 100 and "big" or "small"}: {v1_1} ({v2_1} skipped)`)
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
local v0 = {}
v0.__index = v0

function v0.new()
    return (setmetatable({size = 0, items = {}}, v0))
end

function v0:push(arg2_1)
    self.size += 1
    self.items[self.size] = arg2_1
end

function v0:pop()
    if self.size == 0 then
        return nil
    end
    local v1_1 = self.items[self.size]
    self.items[self.size] = nil
    self.size -= 1
    return v1_1
end

return v0
```

</td>
</tr>
</table>
