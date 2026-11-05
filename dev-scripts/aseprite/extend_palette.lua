-- Append the opaque colours of a sprite that are not yet in a palette file to the end of that palette
-- (existing indexes do not move), then give the sprite the extended palette.
--
-- Usage:
--   aseprite -b --script-param palette=<palette.aseprite> --script-param sprite=<sprite.aseprite> \
--     --script dev-scripts/aseprite/extend_palette.lua
-- The sprite must be in RGB colour mode. Colours are added in order of first use.
local palette_path = app.params["palette"]
local sprite_path = app.params["sprite"]
if not palette_path or not sprite_path then
  print("✗ Pass palette and sprite with --script-param")
  return
end

local palette_spr = Sprite { fromFile = palette_path }
local spr = Sprite { fromFile = sprite_path }
if spr.colorMode ~= ColorMode.RGB then
  print("✗ " .. sprite_path .. " is not in RGB colour mode")
  return
end

local palette = palette_spr.palettes[1]
local known = {}
for i = 0, #palette - 1 do
  local c = palette:getColor(i)
  known[string.format("%02x%02x%02x", c.red, c.green, c.blue)] = true
end

local added = {}
for _, cel in ipairs(spr.cels) do
  for it in cel.image:pixels() do
    local v = it()
    if app.pixelColor.rgbaA(v) > 0 then
      local r, g, b = app.pixelColor.rgbaR(v), app.pixelColor.rgbaG(v), app.pixelColor.rgbaB(v)
      local key = string.format("%02x%02x%02x", r, g, b)
      if not known[key] then
        known[key] = true
        table.insert(added, Color { r = r, g = g, b = b, a = 255 })
      end
    end
  end
end

local first_new = #palette
palette:resize(#palette + #added)
for i, color in ipairs(added) do
  palette:setColor(first_new + i - 1, color)
  print(string.format("  %d #%02x%02x%02x", first_new + i - 1, color.red, color.green, color.blue))
end

palette_spr:setPalette(palette)
palette_spr:saveAs(palette_path)
spr:setPalette(palette)
spr:saveAs(sprite_path)
print(string.format("✓ %d colours added, palette now has %d", #added, #palette))
