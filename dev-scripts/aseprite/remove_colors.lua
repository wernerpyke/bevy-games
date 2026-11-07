-- Make every pixel of the given exact colours fully transparent, in every cel of an RGB .aseprite file. Used to clear
-- stray marks left from a source sheet. Without apply=1 it only reports what it would clear and writes nothing.
--
-- Usage (see dev-scripts/aseprite.sh):
--   ./dev-scripts/aseprite.sh remove_colors <file.aseprite> colors=684f49,5a4844 [apply=1] [out=<file>]
--
--   colors: comma separated RRGGBB hex colours.
--   apply:  1 saves the file (to out=, default the input file). Anything else is a dry run.
local spr = app.sprite
local colors_param = app.params["colors"]
if not spr or not colors_param then
  print("✗ Usage: remove_colors <file.aseprite> colors=<RRGGBB,...> [apply=1] [out=<file>]")
  return
end
if spr.colorMode ~= ColorMode.RGB then
  print("✗ The file is not in RGB colour mode")
  return
end

local counts = {}
for hex in string.gmatch(colors_param, "(%x%x%x%x%x%x)") do
  counts[hex:lower()] = 0
end

local clear = app.pixelColor.rgba(0, 0, 0, 0)
for _, cel in ipairs(spr.cels) do
  for it in cel.image:pixels() do
    local v = it()
    if app.pixelColor.rgbaA(v) > 0 then
      local key = string.format("%02x%02x%02x", app.pixelColor.rgbaR(v), app.pixelColor.rgbaG(v), app.pixelColor.rgbaB(v))
      if counts[key] then
        counts[key] = counts[key] + 1
        it(clear)
      end
    end
  end
end

for key, n in pairs(counts) do
  print(string.format("  %s: %d px", key, n))
end

if app.params["apply"] == "1" then
  local out = app.params["out"] or spr.filename
  spr:saveAs(out)
  print("✓ Saved " .. out)
else
  print("Dry run, nothing written. Add apply=1 to save.")
end
