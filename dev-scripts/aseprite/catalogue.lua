-- Build a navigable catalogue of a tile sheet: an .aseprite file with the sheet at 1:1 (never scaled), a margin with
-- column numbers across the top and row numbers down the left, and a grid. Zoom in Aseprite to read it.
-- Layers (bottom to top): Backdrop (solid colour, so transparent areas are visible), Sheet, Grid, Rulers.
-- Columns and rows are counted from 0 at the top left, so tile (c, r) starts at pixel (c * tile, r * tile).
--
-- Usage:
--   ./dev-scripts/aseprite.sh catalogue <sheet.png> tile=32 out=<file.aseprite>
-- Optional: tilew and tileh for tiles that are not square, backdrop=RRGGBB (default e8e0d0).
local out = app.params["out"]
local tile_w = tonumber(app.params["tilew"] or app.params["tile"])
local tile_h = tonumber(app.params["tileh"] or app.params["tile"])
if not (app.sprite and out and tile_w and tile_h) then
  print("✗ Pass the sheet as the input file, out and tile (or tilew and tileh) with --script-param")
  return
end

local backdrop_hex = app.params["backdrop"] or "e8e0d0"
local function hex(h, a)
  return Color { r = tonumber(h:sub(1, 2), 16), g = tonumber(h:sub(3, 4), 16), b = tonumber(h:sub(5, 6), 16), a = a or 255 }
end

local sheet = app.sprite
local sheet_img = Image(sheet.spec)
sheet_img:drawSprite(sheet, 1)
local sw, sh = sheet.width, sheet.height
local cols, rows = sw // tile_w, sh // tile_h

local MARGIN = 20
local SCALE = 2 -- the 3x5 digit font is drawn at 2x, a label only
local DIGITS = {
  ["0"] = "111101101101111", ["1"] = "010110010010111", ["2"] = "111001111100111", ["3"] = "111001111001111",
  ["4"] = "101101111001001", ["5"] = "111100111001111", ["6"] = "111100111101111", ["7"] = "111001001001001",
  ["8"] = "111101111101111", ["9"] = "111101111001111",
}

local spr = Sprite(sw + MARGIN, sh + MARGIN, ColorMode.RGB)
spr.filename = out
local function new_layer(name)
  local layer = spr.layers[1]
  if #spr.cels > 0 or layer.name ~= "Layer" then
    layer = spr:newLayer()
  end
  layer.name = name
  return layer
end

local function put(layer, img, x, y)
  spr:newCel(layer, 1, img, Point(x, y))
end

-- Backdrop
local bd = new_layer("Backdrop")
local bd_img = Image(sw + MARGIN, sh + MARGIN)
bd_img:clear(hex(backdrop_hex))
put(bd, bd_img, 0, 0)

-- Sheet
put(new_layer("Sheet"), sheet_img, MARGIN, MARGIN)

-- Grid: one pixel line on each tile boundary, a stronger one every 8 tiles
local grid_img = Image(sw + 1, sh + 1)
local thin, thick = hex("000000", 70), hex("d00000", 170)
for c = 0, cols do
  local x = math.min(c * tile_w, sw)
  local color = (c % 8 == 0) and thick or thin
  for y = 0, sh do grid_img:putPixel(x, y, color) end
end
for r = 0, rows do
  local y = math.min(r * tile_h, sh)
  local color = (r % 8 == 0) and thick or thin
  for x = 0, sw do grid_img:putPixel(x, y, color) end
end
put(new_layer("Grid"), grid_img, MARGIN, MARGIN)

-- Rulers
local rul_img = Image(sw + MARGIN, sh + MARGIN)
local ink, ruler_bg = hex("101010"), hex("ffffff")
for x = 0, sw + MARGIN - 1 do for y = 0, MARGIN - 1 do rul_img:putPixel(x, y, ruler_bg) end end
for x = 0, MARGIN - 1 do for y = 0, sh + MARGIN - 1 do rul_img:putPixel(x, y, ruler_bg) end end

local function draw_text(img, text, cx, cy)
  local glyph_w = 3 * SCALE
  local total = #text * glyph_w + (#text - 1) * SCALE
  local x0 = cx - total // 2
  local y0 = cy - (5 * SCALE) // 2
  for i = 1, #text do
    local bits = DIGITS[text:sub(i, i)]
    local gx = x0 + (i - 1) * (glyph_w + SCALE)
    for p = 0, 14 do
      if bits:sub(p + 1, p + 1) == "1" then
        local px, py = gx + (p % 3) * SCALE, y0 + (p // 3) * SCALE
        for dx = 0, SCALE - 1 do for dy = 0, SCALE - 1 do img:putPixel(px + dx, py + dy, ink) end end
      end
    end
  end
end

for c = 0, cols - 1 do
  draw_text(rul_img, tostring(c), MARGIN + c * tile_w + tile_w // 2, MARGIN // 2)
end
for r = 0, rows - 1 do
  draw_text(rul_img, tostring(r), MARGIN // 2, MARGIN + r * tile_h + tile_h // 2)
end
put(new_layer("Rulers"), rul_img, 0, 0)

spr:saveAs(out)
print(string.format("✓ %s: %d columns (0-%d), %d rows (0-%d), tile %dx%d", out, cols, cols - 1, rows, rows - 1, tile_w, tile_h))
