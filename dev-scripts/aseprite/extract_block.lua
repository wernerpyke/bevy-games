-- Cut one block of tiles out of a sheet (see tile-sheet-catalogue.md), anchor the art inside the block's cell space,
-- scale it with nearest neighbour and save it as a one-frame .aseprite file. The sheet is never changed.
--
-- Usage (see dev-scripts/aseprite.sh, the input file is the original sheet, not the catalogue):
--   ./dev-scripts/aseprite.sh extract_block <sheet.png> col=20 row=4 cols=2 rows=2 tile=32 scale=2 \
--     anchor=trunk out=<file.aseprite>
--
--   col, row     top left tile of the block, counted from 0 at the top left of the sheet
--   cols, rows   size of the block in tiles (the cell space the sprite is anchored in)
--   tile         tile size in pixels (or tilew and tileh)
--   scale        whole-number scale applied last, nearest neighbour (default 1)
--   anchor       bottom: the lowest opaque pixel row moves to the bottom of the cell space, left and right stay.
--                trunk:  the lowest opaque row is the trunk foot. The foot moves to the bottom middle of the
--                        cell space. The canvas is widened (never narrowed) so no art is cut off, and the foot
--                        stays on the exact middle column of the canvas.
--                none:   the block is cut as it is.
--
-- The canvas is never smaller than the cell space. When the art is taller than the cell space, the canvas grows
-- upward, so the bottom of the canvas is always the bottom of the cell space. Prints the result.
local sheet = app.sprite
local out = app.params["out"]
local col, row = tonumber(app.params["col"]), tonumber(app.params["row"])
local cols, rows = tonumber(app.params["cols"] or "1"), tonumber(app.params["rows"] or "1")
local tw = tonumber(app.params["tilew"] or app.params["tile"])
local th = tonumber(app.params["tileh"] or app.params["tile"])
local scale = tonumber(app.params["scale"] or "1")
local anchor = app.params["anchor"] or "bottom"
if not (sheet and out and col and row and tw and th) then
  print("✗ Pass the sheet as the input file, and out, col, row and tile with --script-param")
  return
end

local src = Image(sheet.spec)
src:drawSprite(sheet, 1)
local bx, by, bw, bh = col * tw, row * th, cols * tw, rows * th
if bx + bw > src.width or by + bh > src.height then
  print("✗ The block is outside the sheet")
  return
end

local block = Image(bw, bh, src.colorMode)
block:drawImage(src, Point(-bx, -by))

-- bounds of the opaque pixels, and the span of the lowest opaque row
local x1, y1, x2, y2 = bw, bh, -1, -1
for it in block:pixels() do
  if app.pixelColor.rgbaA(it()) > 0 then
    x1, x2 = math.min(x1, it.x), math.max(x2, it.x)
    y1, y2 = math.min(y1, it.y), math.max(y2, it.y)
  end
end
if x2 < 0 then
  print("✗ The block has no opaque pixels")
  return
end
local foot_min, foot_max = bw, -1
for x = 0, bw - 1 do
  if app.pixelColor.rgbaA(block:getPixel(x, y2)) > 0 then
    foot_min, foot_max = math.min(foot_min, x), math.max(foot_max, x)
  end
end

local art_h = y2 + 1 - y1
local canvas_w, canvas_h = bw, math.max(bh, art_h)
local dx, dy = 0, canvas_h - (y2 + 1) -- the lowest art row goes to the bottom of the canvas
local note = ""
if anchor == "none" then
  canvas_h, dy = bh, 0
elseif anchor == "trunk" then
  local foot_c = (foot_min + foot_max + 1) / 2 -- middle of the foot, in pixel edges
  local reach = math.max(foot_c - x1, x2 + 1 - foot_c)
  local half = math.max(bw / 2, math.ceil(reach))
  canvas_w = half * 2
  dx = math.floor(half - foot_c + 0.5)
  note = string.format(", trunk foot columns %d-%d", foot_min, foot_max)
elseif anchor ~= "bottom" then
  print("✗ anchor must be bottom, trunk or none")
  return
end

local result = Sprite(canvas_w, canvas_h, ColorMode.RGB)
result.filename = out
result.layers[1].name = "Sprite"
local img = Image(canvas_w, canvas_h)
img:drawImage(block, Point(dx, dy))
result:newCel(result.layers[1], 1, img, Point(0, 0))

if scale ~= 1 then
  app.command.SpriteSize { ui = false, width = canvas_w * scale, height = canvas_h * scale, lockRatio = false, method = "nearest" }
end
result:saveAs(out)
print(string.format("✓ %s: %dx%d (cell space %dx%d, art %dx%d%s), scale %d -> %dx%d, anchor %s",
  out, canvas_w, canvas_h, bw, bh, x2 + 1 - x1, art_h, note, scale, canvas_w * scale, canvas_h * scale, anchor))
