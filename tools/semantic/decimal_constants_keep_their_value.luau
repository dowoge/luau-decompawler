-- Decimals that are fractions over 255, 1920, 1080 or 60 (or over nothing
-- common) must come back as the very same doubles, whatever spelling is printed.
local function exact(x)
	return string.format("%.17g", x)
end

local Color = { r = 0.25098039215686274, g = 0.39215686274509803, b = 0.9019607843137255 }
print("color", exact(Color.r), exact(Color.g), exact(Color.b), math.round(Color.b * 255))

local layout = { 0.5208333333333334, 0.6481481481481481, 0.23148148148148148, 0.78125 }
local px = {}
for i, scale in ipairs(layout) do
	px[i] = exact(scale) .. "@" .. exact(scale * (if i % 2 == 1 then 1920 else 1080))
end
print("layout", table.concat(px, " "))

local durations = { 0.2833333333333333, 0.6833333333333333, 1.2833333333333334, 0.4166666666666667, 0.36666666666666664, 0.8333333333333334 }
local frames = 0
for _, d in durations do
	frames += d * 60
end
print("frames", exact(frames), exact(durations[5]), exact(durations[6]))

local ratios = { 0.38461538461538464, 0.1111111111111111, 0.041666666666666664 }
print("ratios", exact(ratios[1]), exact(ratios[2] * 9), exact(1 / ratios[3]))
print("folded", exact(3 * 0.25098039215686274), 0.6481481481481481 * 1080 == 700, exact(0.8333333333333334 - 0.5))
