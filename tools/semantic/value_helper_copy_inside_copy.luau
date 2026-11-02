-- A value helper whose copy holds another helper's copy in one branch, inlined into
-- several callers: each caller gets exactly what the nested calls would give.
local remote = select("#", ...) > 3
local Services = {
	Grid = {
		Query = function(_, req)
			return { "cell-" .. req.Zone }
		end,
	},
	Nav = {
		Query = function(_, zone)
			return {
				{
					Point = function()
						return "wp-" .. zone
					end,
				},
			}
		end,
	},
}
local cache = { "home" }

local function cells(req)
	return Services.Grid:Query(req)
end

local function waypoints(req)
	if remote then
		local list = {}
		for i, wp in ipairs(Services.Nav:Query(req.Zone)) do
			list[i] = wp:Point()
		end
		return list
	end
	table.insert(cache, req.Kind)
	return { cache[1], req.Limit, #cache }
end

local function route(req)
	if req.Kind == "walk" then
		return (waypoints(req))
	end
	return cells(req)
end

local function render(list, title)
	local bits = {}
	for _, v in ipairs(list) do
		bits[#bits + 1] = tostring(v)
	end
	return title .. "[" .. table.concat(bits, " ") .. "]"
end

local Planner = {}
function Planner.short(req)
	local zone = req.Zone
	local path = route(req)
	print("short", zone)
	return render(path, "s" .. zone)
end
function Planner.long(req)
	local path = route(req)
	local limit = req.Limit
	print("long", limit, #path)
	return render(path, "l" .. limit)
end
function Planner.custom(req)
	local path = if req.Kind == "walking" then waypoints(req) else cells(req)
	print("custom", #path)
	return render(path, "c")
end

print(Planner.short({ Zone = 2, Kind = "walk", Limit = 5 }))
print(Planner.short({ Zone = 3, Kind = "fly", Limit = 5 }))
print(Planner.long({ Zone = 4, Kind = "fly", Limit = 9 }))
print(Planner.long({ Zone = 5, Kind = "walk", Limit = 1 }))
print(Planner.custom({ Zone = 6, Kind = "walking", Limit = 2 }))
print(Planner.custom({ Zone = 7, Kind = "walk", Limit = 2 }))
print(#cache)
