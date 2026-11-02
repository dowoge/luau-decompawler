local M = {}
function M.bubble(t)
    local n = #t
    for i = 1, n - 1 do
        local swapped = false
        for j = 1, n - i do
            if t[j] > t[j + 1] then
                t[j], t[j + 1] = t[j + 1], t[j]
                swapped = true
            end
        end
        if not swapped then break end
    end
    return t
end
function M.quick(t, lo, hi)
    lo = lo or 1
    hi = hi or #t
    if lo < hi then
        local p = t[hi]
        local i = lo - 1
        for j = lo, hi - 1 do
            if t[j] <= p then
                i += 1
                t[i], t[j] = t[j], t[i]
            end
        end
        t[i + 1], t[hi] = t[hi], t[i + 1]
        M.quick(t, lo, i)
        M.quick(t, i + 2, hi)
    end
    return t
end
function M.bsearch(t, x)
    local lo, hi = 1, #t
    while lo <= hi do
        local mid = (lo + hi) // 2
        if t[mid] == x then
            return mid
        elseif t[mid] < x then
            lo = mid + 1
        else
            hi = mid - 1
        end
    end
    return nil
end
function M.merge(a, b)
    local out, i, j = {}, 1, 1
    while i <= #a and j <= #b do
        if a[i] <= b[j] then
            table.insert(out, a[i])
            i += 1
        else
            table.insert(out, b[j])
            j += 1
        end
    end
    while i <= #a do table.insert(out, a[i]) i += 1 end
    while j <= #b do table.insert(out, b[j]) j += 1 end
    return out
end
function M.sieve(n)
    local comp, primes = {}, {}
    for i = 2, n do
        if not comp[i] then
            table.insert(primes, i)
            for j = i * i, n, i do comp[j] = true end
        end
    end
    return primes
end
function M.gcd(a, b) while b ~= 0 do a, b = b, a % b end return a end
function M.fib(n) local a, b = 0, 1 for _ = 1, n do a, b = b, a + b end return a end
function M.bfs(g, s)
    local seen, q, order = {[s] = true}, {s}, {}
    local head = 1
    while head <= #q do
        local v = q[head]
        head += 1
        table.insert(order, v)
        for _, w in ipairs(g[v] or {}) do
            if not seen[w] then
                seen[w] = true
                table.insert(q, w)
            end
        end
    end
    return order
end
function M.dfs(g, v, seen, out)
    seen = seen or {}
    out = out or {}
    if seen[v] then return out end
    seen[v] = true
    table.insert(out, v)
    for _, w in ipairs(g[v] or {}) do M.dfs(g, w, seen, out) end
    return out
end
function M.astar(grid, sx, sy, gx, gy)
    local function key(x, y) return x * 1000 + y end
    local open, g, parent = {{x = sx, y = sy, f = 0}}, {[key(sx, sy)] = 0}, {}
    while #open > 0 do
        local best = 1
        for i, n in ipairs(open) do
            if n.f < open[best].f then best = i end
        end
        local cur = table.remove(open, best)
        if cur.x == gx and cur.y == gy then
            local len, k = 0, key(gx, gy)
            while parent[k] do
                k = parent[k]
                len += 1
            end
            return len
        end
        for _, d in ipairs({{1, 0}, {-1, 0}, {0, 1}, {0, -1}}) do
            local nx, ny = cur.x + d[1], cur.y + d[2]
            local row = grid[ny]
            if row and row[nx] == 0 then
                local cost = g[key(cur.x, cur.y)] + 1
                if g[key(nx, ny)] == nil or cost < g[key(nx, ny)] then
                    g[key(nx, ny)] = cost
                    parent[key(nx, ny)] = key(cur.x, cur.y)
                    table.insert(open, {x = nx, y = ny, f = cost + math.abs(gx - nx) + math.abs(gy - ny)})
                end
            end
        end
    end
    return -1
end
function M.lcs(a, b)
    local dp = {}
    for i = 0, #a do
        dp[i] = {}
        for j = 0, #b do
            if i == 0 or j == 0 then
                dp[i][j] = 0
            elseif a:sub(i, i) == b:sub(j, j) then
                dp[i][j] = dp[i - 1][j - 1] + 1
            else
                dp[i][j] = math.max(dp[i - 1][j], dp[i][j - 1])
            end
        end
    end
    return dp[#a][#b]
end
function M.heappush(h, v)
    table.insert(h, v)
    local i = #h
    while i > 1 do
        local p = i // 2
        if h[p] <= h[i] then break end
        h[p], h[i] = h[i], h[p]
        i = p
    end
end
function M.heappop(h)
    local top = h[1]
    local last = table.remove(h)
    if #h > 0 then
        h[1] = last
        local i = 1
        while true do
            local l, r, m = i * 2, i * 2 + 1, i
            if l <= #h and h[l] < h[m] then m = l end
            if r <= #h and h[r] < h[m] then m = r end
            if m == i then break end
            h[m], h[i] = h[i], h[m]
            i = m
        end
    end
    return top
end
print(table.concat(M.bubble({3, 1, 2}), ","), table.concat(M.quick({5, 2, 9, 1}), ","), M.bsearch({1, 3, 5}, 3), M.bsearch({1}, 2), table.concat(M.merge({1, 4}, {2, 3}), ","))
print(table.concat(M.sieve(20), ","), M.gcd(12, 18), M.fib(10), table.concat(M.bfs({a = {"b", "c"}, b = {"d"}}, "a"), ","), table.concat(M.dfs({a = {"b", "c"}, b = {"a"}}, "a"), ","))
print(M.astar({{0, 0, 0}, {1, 1, 0}, {0, 0, 0}}, 1, 1, 1, 3), M.astar({{0, 1}, {1, 0}}, 1, 1, 2, 2), M.lcs("abcde", "ace"))
local h = {} for _, v in ipairs({5, 1, 4}) do M.heappush(h, v) end print(M.heappop(h), M.heappop(h), M.heappop(h))
