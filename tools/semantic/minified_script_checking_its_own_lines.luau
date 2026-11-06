-- A minified script (all on one line) that checks its own line numbers, as
-- obfuscators do: written over many lines it takes another way. It must come
-- back on its one line, the line it was on.
local first = debug.info(1, "l") local function check(n) local now = debug.info(1, "l") if now ~= first then return "moved", n end return "same line", n * 2 end local t = {} for i = 1, 3 do t[i] = select(2, check(i)) end print(check(5)) print(first, table.concat(t, ","))
