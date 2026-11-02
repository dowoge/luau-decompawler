local M = {}
function M.split(s, sep)
    local out = {}
    local start = 1
    while true do
        local a, b = string.find(s, sep, start, true)
        if a == nil then
            table.insert(out, string.sub(s, start))
            break
        end
        table.insert(out, string.sub(s, start, a - 1))
        start = b + 1
    end
    return out
end
function M.trim(s) return (s:gsub("^%s+", ""):gsub("%s+$", "")) end
function M.count(s, c) local n = 0 for i = 1, #s do if s:sub(i, i) == c then n += 1 end end return n end
function M.reverse(s) local r = "" for i = #s, 1, -1 do r ..= s:sub(i, i) end return r end
function M.words(s) local w = {} for x in s:gmatch("%S+") do w[#w + 1] = x end return w end
function M.fmt(a, b) return string.format("%d-%s-%.2f", a, b, a / 3) end
function M.caesar(s, k)
    local out = {}
    for i = 1, #s do
        local c = s:byte(i)
        if c >= 97 and c <= 122 then
            c = (c - 97 + k) % 26 + 97
        elseif c >= 65 and c <= 90 then
            c = (c - 65 + k) % 26 + 65
        end
        out[i] = string.char(c)
    end
    return table.concat(out)
end
function M.ispal(s)
    local i, j = 1, #s
    while i < j do
        if s:sub(i, i) ~= s:sub(j, j) then return false end
        i += 1
        j -= 1
    end
    return true
end
function M.tokenize(src)
    local toks = {}
    local i = 1
    while i <= #src do
        local c = src:sub(i, i)
        if c:match("%s") then
            i += 1
        elseif c:match("%d") then
            local j = i
            while j <= #src and src:sub(j, j):match("%d") do j += 1 end
            table.insert(toks, {kind = "num", value = tonumber(src:sub(i, j - 1))})
            i = j
        elseif c:match("%a") then
            local j = i
            while j <= #src and src:sub(j, j):match("[%w_]") do j += 1 end
            table.insert(toks, {kind = "id", value = src:sub(i, j - 1)})
            i = j
        else
            table.insert(toks, {kind = "op", value = c})
            i += 1
        end
    end
    return toks
end
function M.escape(s)
    return (s:gsub('[%c"\\]', function(c)
        if c == '"' then return '\\"' end
        if c == "\\" then return "\\\\" end
        return string.format("\\%03d", c:byte())
    end))
end
print(#M.split("a,b,,c", ","), M.trim("  x y  "), M.count("banana", "a"), M.reverse("abc"), #M.words("a bb  c"), M.fmt(4, "x"))
print(M.caesar("Hello, World", 3), M.ispal("abba"), M.ispal("abc"), #M.tokenize("x1 = 42 + y"), M.tokenize("a 1")[2].value, M.escape('a"b\\c\n'))
