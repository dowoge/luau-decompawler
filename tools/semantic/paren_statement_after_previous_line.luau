-- A statement starting with `(` right after one ending in a name, `)`, `]` or a
-- length operand needs a separator, or it would continue the previous expression.
local queues = {}
local Tween = {}
Tween.__index = Tween

local function lookup(name)
	return queues[name]
end
local function create(name)
	local q = { jobs = {} }
	queues[name] = q
	return q
end

function Tween:cancel()
	self.cancelled = true
end
function Tween:enqueue(queueName)
	self:cancel();
	(lookup(queueName) or create(queueName)).jobs[self.slot] = self
	self.queue = queueName
end

local fade = setmetatable({ slot = 5 }, Tween)
fade:enqueue("fx")
local slide = setmetatable({ slot = 6 }, Tween)
slide:enqueue("fx")
print(queues.fx.jobs[5] == fade, queues.fx.jobs[6] == slide, fade.cancelled, slide.queue)

local missing, target = nil, { props = {} }
local alias = target;
(missing or alias).props.width = 10
local sizes = { 4, 8 }
local first = sizes[2];
(missing or target).props.height = first
local caption = "tween"
local count = #caption;
(missing or target).props.chars = count
local function flush() end
flush(); (missing or target).props.done = true
print(target.props.width, target.props.height, target.props.chars, target.props.done)
