-- Cut a block of equal tiles out of a PNG, such as a tileset, and save each tile as its own PNG. Tiles are read row by
-- row from the top left, and the names are given in the same order. Feed the PNGs to pack_pngs.lua afterwards.
--
-- Usage:
--   aseprite -b --script-param src=<sheet.png> --script-param x=256 --script-param y=0 \
--     --script-param tile=16 --script-param cols=3 --script-param rows=3 \
--     --script-param names=nw,n,ne,w,c,e,sw,s,se --script-param outdir=<folder> \
--     --script dev-scripts/aseprite/cut_tiles.lua
--
-- x and y are the pixel position of the top left corner of the block. tile is the width and height of one tile
-- (use tilew and tileh for tiles that are not square). The folder is created when it does not exist.
local src = app.params["src"]
local outdir = app.params["outdir"]
local names_param = app.params["names"]
local x0 = tonumber(app.params["x"] or "0")
local y0 = tonumber(app.params["y"] or "0")
local tile_w = tonumber(app.params["tilew"] or app.params["tile"])
local tile_h = tonumber(app.params["tileh"] or app.params["tile"])
local cols = tonumber(app.params["cols"])
local rows = tonumber(app.params["rows"])
if not (src and outdir and names_param and tile_w and tile_h and cols and rows) then
  print("✗ Pass src, outdir, names, tile (or tilew and tileh), cols and rows with --script-param")
  return
end

local names = {}
for name in string.gmatch(names_param, "[^,]+") do
  table.insert(names, name)
end
if #names ~= cols * rows then
  print(string.format("✗ %d names for %d tiles", #names, cols * rows))
  return
end

local sheet = Image { fromFile = src }
local right = x0 + cols * tile_w
local bottom = y0 + rows * tile_h
if x0 < 0 or y0 < 0 or right > sheet.width or bottom > sheet.height then
  print(string.format("✗ The block %d,%d to %d,%d is outside the %dx%d image", x0, y0, right, bottom, sheet.width,
    sheet.height))
  return
end

app.fs.makeAllDirectories(outdir)
for row = 0, rows - 1 do
  for col = 0, cols - 1 do
    local name = names[row * cols + col + 1]
    local tile = Image(tile_w, tile_h, sheet.colorMode)
    tile:drawImage(sheet, Point(-(x0 + col * tile_w), -(y0 + row * tile_h)))
    local path = app.fs.joinPath(outdir, name .. ".png")
    tile:saveAs(path)
    print(string.format("%s from %d,%d", path, x0 + col * tile_w, y0 + row * tile_h))
  end
end
print(string.format("✓ %d tiles of %dx%d in %s", #names, tile_w, tile_h, outdir))
