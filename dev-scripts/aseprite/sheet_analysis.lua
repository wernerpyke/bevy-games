-- Analyse a sprite sheet PNG (equal frames in a grid, as in a sprite pack): per frame the bounds of the
-- opaque pixels and the number of unique colours, then the list of colours in the whole sheet.
-- Read only.
--
-- Usage (see dev-scripts/aseprite.sh):
--   aseprite -b <sheet.png> --script-param frame=96x64 --script dev-scripts/aseprite/sheet_analysis.lua
--
--   frame: size of one frame, WxH. Frames are read left to right, then top to bottom.
-- Bounds are 0-based pixel coordinates inside the frame, both ends included. Pixels with alpha 0 are empty.
local spr = app.sprite
if not spr then
  print("✗ No image open")
  return
end

local frame_w, frame_h = string.match(app.params["frame"] or "", "^(%d+)x(%d+)$")
frame_w, frame_h = tonumber(frame_w), tonumber(frame_h)
if not frame_w then
  print("✗ Pass frame=<W>x<H> with --script-param")
  return
end
if spr.colorMode ~= ColorMode.RGB then
  print("✗ The sheet must be an RGB PNG")
  return
end

local image = Image(spr.spec)
image:clear()
image:drawSprite(spr, 1)
local cols, rows = spr.width // frame_w, spr.height // frame_h
print(string.format("%s: %dx%d, %d columns x %d rows of %dx%d", app.fs.fileName(spr.filename), spr.width, spr.height, cols, rows, frame_w, frame_h))

local all = {}
for row = 0, rows - 1 do
  for col = 0, cols - 1 do
    local min_x, min_y, max_x, max_y = frame_w, frame_h, -1, -1
    local own = {}
    for y = 0, frame_h - 1 do
      for x = 0, frame_w - 1 do
        local value = image:getPixel(col * frame_w + x, row * frame_h + y)
        if app.pixelColor.rgbaA(value) > 0 then
          min_x, min_y = math.min(min_x, x), math.min(min_y, y)
          max_x, max_y = math.max(max_x, x), math.max(max_y, y)
          local key = string.format("%02x%02x%02x%02x", app.pixelColor.rgbaR(value), app.pixelColor.rgbaG(value), app.pixelColor.rgbaB(value), app.pixelColor.rgbaA(value))
          own[key] = true
          all[key] = true
        end
      end
    end
    local n = row * cols + col + 1
    local unique = 0
    for _ in pairs(own) do
      unique = unique + 1
    end
    if max_x < 0 then
      print(string.format("  %3d: empty", n))
    else
      print(string.format("  %3d: x %d-%d, y %d-%d, size %dx%d, centre x %.1f, feet y %d, %d colours", n, min_x, max_x, min_y, max_y, max_x - min_x + 1, max_y - min_y + 1, (min_x + max_x) / 2, max_y, unique))
    end
  end
end

local list = {}
for key in pairs(all) do
  list[#list + 1] = "#" .. key
end
table.sort(list)
print(string.format("Unique colours in the sheet (%d): %s", #list, table.concat(list, " ")))
