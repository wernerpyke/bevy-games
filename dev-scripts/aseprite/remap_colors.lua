-- Replace exact colours in an open RGB .aseprite file, in every cel. Used to nudge art onto a palette: map the colours
-- that have a close palette match, then run extend_palette.lua for the few that have none. Without apply=1 it only
-- reports what it would change and writes nothing.
--
-- Usage (see dev-scripts/aseprite.sh):
--   ./dev-scripts/aseprite.sh remap_colors <file.aseprite> map=080d16:1b141c,4b6396:425faa [apply=1] [out=<file>]
--
--   map:   comma separated from:to pairs of RRGGBB hex colours. Alpha is kept.
--   apply: 1 saves the file (to out=, default the input file). Anything else is a dry run.
-- The report lists each pair with its pixel count, and the colours of the file that no pair touches.
local spr = app.sprite
local map_param = app.params["map"]
if not spr or not map_param then
  print("✗ Usage: remap_colors <file.aseprite> map=<from:to,...> [apply=1] [out=<file>]")
  return
end
if spr.colorMode ~= ColorMode.RGB then
  print("✗ The file is not in RGB colour mode")
  return
end

local function parse(h)
  return tonumber(h:sub(1, 2), 16), tonumber(h:sub(3, 4), 16), tonumber(h:sub(5, 6), 16)
end

local map, counts = {}, {}
for from, to in string.gmatch(map_param, "(%x%x%x%x%x%x):(%x%x%x%x%x%x)") do
  map[from:lower()] = to:lower()
  counts[from:lower()] = 0
end

local untouched = {}
for _, cel in ipairs(spr.cels) do
  local img = cel.image
  for it in img:pixels() do
    local v = it()
    local a = app.pixelColor.rgbaA(v)
    if a > 0 then
      local key = string.format("%02x%02x%02x", app.pixelColor.rgbaR(v), app.pixelColor.rgbaG(v), app.pixelColor.rgbaB(v))
      local to = map[key]
      if to then
        counts[key] = counts[key] + 1
        local r, g, b = parse(to)
        it(app.pixelColor.rgba(r, g, b, a))
      else
        untouched[key] = (untouched[key] or 0) + 1
      end
    end
  end
end

for from, to in pairs(map) do
  print(string.format("  %s -> %s: %d px", from, to, counts[from]))
end
for key, n in pairs(untouched) do
  print(string.format("  unchanged %s: %d px", key, n))
end

if app.params["apply"] == "1" then
  local out = app.params["out"] or spr.filename
  spr:saveAs(out)
  print("✓ saved " .. out)
else
  print("Dry run, nothing written. Add apply=1 to save.")
end
