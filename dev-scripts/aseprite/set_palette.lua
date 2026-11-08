-- Give an open RGB .aseprite file the palette of another file (the game palette), without changing any pixel. Refuses
-- when a visible colour of the file is not in that palette: nothing is saved then. Without apply=1 it only reports.
--
-- Usage (see dev-scripts/aseprite.sh):
--   ./dev-scripts/aseprite.sh set_palette <file.aseprite> palette=<game.aseprite> [apply=1]
local spr = app.sprite
local palette_path = app.params["palette"]
if not spr or not palette_path then
  print("✗ Usage: set_palette <file.aseprite> palette=<palette.aseprite> [apply=1]")
  return
end
if spr.colorMode ~= ColorMode.RGB then
  print("✗ The file is not in RGB colour mode")
  return
end
local palette_spr = Sprite { fromFile = palette_path }
local palette = palette_spr.palettes[1]
local known = {}
for i = 0, #palette - 1 do
  local c = palette:getColor(i)
  known[string.format("%02x%02x%02x", c.red, c.green, c.blue)] = true
end
local missing = 0
for _, cel in ipairs(spr.cels) do
  for it in cel.image:pixels() do
    local v = it()
    if app.pixelColor.rgbaA(v) > 0 then
      local key = string.format("%02x%02x%02x", app.pixelColor.rgbaR(v), app.pixelColor.rgbaG(v), app.pixelColor.rgbaB(v))
      if not known[key] then missing = missing + 1 end
    end
  end
end
if missing > 0 then
  print("✗ " .. missing .. " pixels use colours that are not in the palette, not changed")
  return
end
if app.params["apply"] ~= "1" then
  print("✓ would set a " .. #palette .. " colour palette (dry run, add apply=1)")
  return
end
spr:setPalette(palette)
spr:saveAs(spr.filename)
print("✓ palette set, " .. #palette .. " colours")
