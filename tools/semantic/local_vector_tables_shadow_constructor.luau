-- Script-local constructor tables (one a `Vector3` stand-in) written on one line
-- each: -O2 inlines their `.new`, and every result must keep its own fields.
local Region = { new = function(lo, hi) return { Min = lo, Max = hi } end }
local Vector3 = { new = function(x, y, z) return { X = x, Y = y, Z = z } end }
local zone = Region.new(-8, 8)
local spawn = Vector3.new(-5, 9, 2)
local goal = Vector3.new(7, 1, 40)
print("points", zone.Min, zone.Max, spawn.X, spawn.Y, spawn.Z, goal.Z)
