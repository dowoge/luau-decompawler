-- `t.Send(name, { Text = f() })`: a field read as the callee, then a call
-- inside a table constructor argument. The callee came out as a local of
-- its own (`local Send = t.Send` before the call, twice in one block) as
-- though the call inside the table could come between; Lua reads the callee
-- before the arguments either way. The calls here swap t.Send while the
-- arguments are made: each call must still reach the function read first.
local log = {}
local t = {}
function t.Send(name, data) log[#log + 1] = "old " .. name .. " " .. tostring(data.Text or data[1] or data.inner.v) end
local function swap(v)
    t.Send = function(name) log[#log + 1] = "new " .. name end
    return v
end
t.Send("a", { Text = swap("x") })
t.Send("b", { Text = "y" })
function t.Send(name, data) log[#log + 1] = "old " .. name .. " " .. tostring(data.Text or data[1] or data.inner.v) end
t.Send("c", { swap("z") })
function t.Send(name, data) log[#log + 1] = "old " .. name .. " " .. tostring(data.Text or data[1] or data.inner.v) end
t.Send("d", { inner = { v = swap("w") } })
local s = t.Send
function t.Send(name, data) log[#log + 1] = "old " .. name .. " " .. tostring(data.Text or data[1] or data.inner.v) end
s("e", { Text = swap("q") })
print(table.concat(log, ", "))
