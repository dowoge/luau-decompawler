-- A local function that names itself, made in a loop and called there once:
-- -O2 inlines the call; rebuilt, it must keep its argument and both results.
local function pings()
	local log = {}
	local step = 0
	while step < 2 do
		step += 1
		local function ping(n)
			return ping, n
		end
		local again, n = ping(step)
		log[#log + 1] = tostring(again == ping) .. n
	end
	print(table.concat(log, " "))
end
pings()

local function heard(limit)
	local hits = {}
	for tick = 1, limit do
		local function echo(t)
			return echo, t
		end
		local _, back = echo(tick)
		hits[tick] = back
	end
	print(hits[1], hits[2])
end
heard(2)

local function sizes(node)
	local out = {}
	for _, child in node do
		if type(child) == "table" then
			local function count(t)
				local n = 0
				for _, v in t do
					n += if type(v) == "table" then count(v) else 1
				end
				return n
			end
			out[#out + 1] = count(child)
		else
			out[#out + 1] = "leaf"
		end
	end
	return table.concat(out, ",")
end
print(sizes({ { 1, { 2, 3 } }, 7, { {} } }))

local function relink()
	local kept = {}
	for i = 1, 2 do
		local link
		link = function()
			return link
		end
		kept[i] = link
		link = i * 5
		print(kept[i]() == kept[i], link)
	end
end
relink()
