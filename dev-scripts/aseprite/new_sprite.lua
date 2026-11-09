-- Make a new RGB .aseprite file with the palette of the open file (the game palette), filled with one palette colour.
-- Used to start a background or other flat art at an exact size. Refuses to overwrite an existing file.
--
-- Usage (see dev-scripts/aseprite.sh; the input file is the palette file):
--   ./dev-scripts/aseprite.sh new_sprite games/long-way/assets/art/game.aseprite \
--     out=games/long-way/assets/art/backgrounds/hud.aseprite width=512 height=160 fill=34 layer=Background
--
--   out:    file to write (required)
--   width, height: canvas size in pixels (required)
--   fill:   palette index to fill the first frame with (default: leave it empty)
--   layer:  name of the layer (default Layer 1)
--   frames: number of (empty) frames (default 1). Only frame 1 gets the fill.
--   tags:   comma separated tag names, one per frame in order, for example tags=Full,Empty (needs frames=<same count>)
local palette_spr = app.sprite
local out = app.params["out"]
local width = tonumber(app.params["width"])
local height = tonumber(app.params["height"])
if not palette_spr or not out or not width or not height then
  print("✗ Usage: new_sprite <palette.aseprite> out=<file> width=<n> height=<n> [fill=<index>] [layer=<name>]")
  return
end
if app.fs.isFile(out) then
  print("✗ " .. out .. " exists, not overwriting")
  return
end

local palette = palette_spr.palettes[1]
local spr = Sprite(width, height, ColorMode.RGB)
spr:setPalette(palette)
local layer = spr.layers[1]
layer.name = app.params["layer"] or "Layer 1"

local fill = tonumber(app.params["fill"])
if fill then
  local colour = palette:getColor(fill)
  local image = Image(width, height, ColorMode.RGB)
  image:clear(colour)
  spr:newCel(layer, 1, image, Point(0, 0))
end

local frames = tonumber(app.params["frames"]) or 1
for _ = 2, frames do
  spr:newEmptyFrame()
end
local tags = app.params["tags"]
if tags then
  local names = {}
  for name in string.gmatch(tags, "[^,]+") do
    names[#names + 1] = name
  end
  if #names ~= frames then
    print(string.format("✗ tags= has %d names but frames=%d, nothing saved", #names, frames))
    return
  end
  for i, name in ipairs(names) do
    spr:newTag(i, i).name = name
  end
end

spr:saveAs(out)
print(string.format("✓ %s: %dx%d, %d palette colours%s", out, width, height, #palette,
  fill and string.format(", filled with index %d (#%02x%02x%02x)", fill, palette:getColor(fill).red,
    palette:getColor(fill).green, palette:getColor(fill).blue) or ""))
