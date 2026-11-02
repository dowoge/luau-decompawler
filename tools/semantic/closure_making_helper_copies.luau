-- Helpers that build and return a new function, copied into callers: each copy makes its
-- own closure over the caller's values, and capture-free literals stay one object.
local function after(owner, step)
	return function(dt, ...)
		owner.elapsed += dt
		return step(owner, dt, ...)
	end
end

local Timer = {}
Timer.__index = Timer
function Timer.tick(self, dt)
	return "tick" .. dt .. "@" .. self.elapsed
end
function Timer.tock(self, dt, extra)
	return "tock" .. dt .. (extra or "")
end
function Timer.create()
	local self = setmetatable({ elapsed = 0 }, Timer)
	self.onTick = after(self, Timer.tick)
	self.onTock = after(self, Timer.tock)
	return self
end

local function schedule(jobs, summarize)
	local results = {}
	local completed = 0
	local function bump()
		completed += 1
	end
	local function wrap(tag)
		return function(v)
			bump()
			return tag .. ":" .. tostring(v) .. "/" .. completed
		end
	end
	for i, job in ipairs(jobs) do
		results[i] = job(wrap("ok"), wrap("fail"))
	end
	return results, summarize(completed)
end

local function curry(fn, first)
	return function(...)
		return fn(first, ...)
	end
end

local function pickHandler(kind, handlers, fallback)
	local handler = fallback
	if handlers[kind] then
		handler = curry(handlers[kind], kind)
	end
	return handler
end

local notes = {}
local function remind(note)
	table.insert(notes, function()
		return note .. " #" .. #notes
	end)
end

local function idle()
	return function()
		return "idle"
	end
end
local Modes = {}
function Modes.day()
	return idle()
end
function Modes.night()
	return idle()
end

local function fade(part, alpha)
	pcall(function() part.Opacity = math.clamp(alpha, 0, 1) end)
end

local panels = {}
function buildPanels(count)
	for i = 1, count do
		local panel = { Opacity = -1 }
		fade(panel, i / 2 - 0.25)
		panels[i] = panel
	end
	fade(panels[count], -3)
	return panels[1].Opacity, panels[count].Opacity
end

local t = Timer.create()
print(t.onTick(2), t.onTick(5), t.onTock(1, "!"), t.elapsed)
local results, summary = schedule({
	function(ok, err)
		return ok(1)
	end,
	function(ok, err)
		return err("oops") .. "+" .. ok(2)
	end,
}, function(n)
	return "done " .. n
end)
print(results[1], results[2], summary)
local handlers = {
	greet = function(kind, name)
		return kind .. " " .. name
	end,
}
local h1 = pickHandler("greet", handlers, print)
local h2 = pickHandler("wave", handlers, function(name)
	return "fallback " .. name
end)
print(h1("ana"), h2("bo"))
remind("water plants")
remind("feed cat")
print(notes[1](), notes[2]())
print(Modes.day() == Modes.day(), Modes.day() == Modes.night(), Modes.night()())
print(buildPanels(3))
