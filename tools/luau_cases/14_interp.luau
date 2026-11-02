-- a small expression language: tokenizer, parser and evaluator
local Lexer = {}
Lexer.__index = Lexer
function Lexer.new(src) return setmetatable({src = src, pos = 1, tok = nil}, Lexer) end
function Lexer:next()
    local s, p = self.src, self.pos
    while p <= #s and s:sub(p, p) == " " do p += 1 end
    if p > #s then
        self.pos = p
        self.tok = {type = "eof"}
        return self.tok
    end
    local c = s:sub(p, p)
    if c:match("%d") then
        local e = p
        while e < #s and s:sub(e + 1, e + 1):match("[%d.]") do e += 1 end
        self.tok = {type = "num", value = tonumber(s:sub(p, e))}
        self.pos = e + 1
    elseif c:match("[%a_]") then
        local e = p
        while e < #s and s:sub(e + 1, e + 1):match("[%w_]") do e += 1 end
        self.tok = {type = "name", value = s:sub(p, e)}
        self.pos = e + 1
    else
        self.tok = {type = c}
        self.pos = p + 1
    end
    return self.tok
end
local PREC = {["+"] = 1, ["-"] = 1, ["*"] = 2, ["/"] = 2, ["^"] = 3}
local RIGHT = {["^"] = true}
local Parser = {}
Parser.__index = Parser
function Parser.new(src)
    local self = setmetatable({lex = Lexer.new(src)}, Parser)
    self.lex:next()
    return self
end
function Parser:primary()
    local t = self.lex.tok
    if t.type == "num" then
        self.lex:next()
        return {op = "num", value = t.value}
    elseif t.type == "name" then
        self.lex:next()
        if self.lex.tok.type == "(" then
            self.lex:next()
            local args = {}
            while self.lex.tok.type ~= ")" do
                table.insert(args, self:expr(0))
                if self.lex.tok.type == "," then self.lex:next() end
            end
            self.lex:next()
            return {op = "call", name = t.value, args = args}
        end
        return {op = "var", name = t.value}
    elseif t.type == "(" then
        self.lex:next()
        local e = self:expr(0)
        if self.lex.tok.type ~= ")" then error("expected )", 0) end
        self.lex:next()
        return e
    elseif t.type == "-" then
        self.lex:next()
        return {op = "neg", arg = self:primary()}
    end
    error("unexpected " .. t.type, 0)
end
function Parser:expr(min)
    local lhs = self:primary()
    while true do
        local op = self.lex.tok.type
        local p = PREC[op]
        if not p or p < min then break end
        self.lex:next()
        local rhs = self:expr(if RIGHT[op] then p else p + 1)
        lhs = {op = op, lhs = lhs, rhs = rhs}
    end
    return lhs
end
local BIN = {
    ["+"] = function(a, b) return a + b end,
    ["-"] = function(a, b) return a - b end,
    ["*"] = function(a, b) return a * b end,
    ["/"] = function(a, b)
        if b == 0 then error("division by zero", 0) end
        return a / b
    end,
    ["^"] = function(a, b) return a ^ b end,
}
local function eval(n, env)
    local op = n.op
    if op == "num" then
        return n.value
    elseif op == "var" then
        local v = env[n.name]
        if v == nil then error("unknown " .. n.name, 0) end
        return v
    elseif op == "neg" then
        return -eval(n.arg, env)
    elseif op == "call" then
        local f = env[n.name]
        local args = {}
        for i, a in ipairs(n.args) do args[i] = eval(a, env) end
        return f(table.unpack(args))
    end
    local f = BIN[op]
    return f(eval(n.lhs, env), eval(n.rhs, env))
end
local function fold(n)
    if n.lhs then
        local l, r = fold(n.lhs), fold(n.rhs)
        if l.op == "num" and r.op == "num" then
            local ok, v = pcall(BIN[n.op], l.value, r.value)
            if ok then return {op = "num", value = v} end
        end
        return {op = n.op, lhs = l, rhs = r}
    elseif n.op == "neg" then
        local a = fold(n.arg)
        if a.op == "num" then return {op = "num", value = -a.value} end
        return {op = "neg", arg = a}
    elseif n.op == "call" then
        local args = table.create(#n.args)
        for i, a in ipairs(n.args) do args[i] = fold(a) end
        return {op = "call", name = n.name, args = args}
    end
    return n
end
local function show(n)
    if n.op == "num" then return tostring(n.value) end
    if n.op == "var" then return n.name end
    if n.op == "neg" then return "-" .. show(n.arg) end
    if n.op == "call" then
        local parts = {}
        for _, a in ipairs(n.args) do table.insert(parts, show(a)) end
        return n.name .. "(" .. table.concat(parts, ", ") .. ")"
    end
    return "(" .. show(n.lhs) .. " " .. n.op .. " " .. show(n.rhs) .. ")"
end
local env = {x = 3, y = 4, max = math.max, sq = function(v) return v * v end}
for _, src in ipairs({"1 + 2 * 3", "(1 + 2) * 3", "2 ^ 3 ^ 2", "-x + y", "max(x, sq(y)) / 2", "1 / 0", "z + 1", "(1", "2 * (3 + x) - 4"}) do
    local ok, r = pcall(function()
        local tree = Parser.new(src):expr(0)
        local f = fold(tree)
        return show(f) .. " = " .. tostring(eval(f, env))
    end)
    print(src, ok, r)
end
