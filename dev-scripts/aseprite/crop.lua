-- Crop a PNG (usually a screenshot from `rust.sh script`) and scale it up with nearest-neighbour.
-- The input file is never changed. The result goes to `out`.
--
-- Usage (see dev-scripts/aseprite.sh):
--   aseprite -b <screenshot.png> --script-param out=<file.png> --script-param scale=4 \
--     --script-param cx=0 --script-param cy=0 --script-param w=200 --script-param h=200 \
--     --script-param view_height=576 --script dev-scripts/aseprite/crop.lua
--
-- Two ways to say which region:
--   World units: cx, cy (centre, world coordinates, y up), w, h (size) and view_height (the game's
--     VIEW_HEIGHT). screen = image centre + world * (image height / view_height), y flipped.
--   Pixels: x, y (top left), w, h, all in image pixels.
-- scale: whole-number zoom of the result, default 1.
local spr = app.sprite
if not spr then
  print("✗ No image open")
  return
end

local function number(name)
  return tonumber(app.params[name])
end

local out = app.params["out"]
if not out then
  print("✗ Pass out with --script-param")
  return
end
local scale = number("scale") or 1
local w, h = number("w"), number("h")
if not (w and h) then
  print("✗ Pass w and h with --script-param")
  return
end

local x, y
if number("cx") and number("cy") then
  local view = number("view_height")
  if not view then
    print("✗ World mode needs view_height")
    return
  end
  local k = spr.height / view
  w, h = math.floor(w * k + 0.5), math.floor(h * k + 0.5)
  x = math.floor(spr.width / 2 + number("cx") * k - w / 2 + 0.5)
  y = math.floor(spr.height / 2 - number("cy") * k - h / 2 + 0.5)
else
  x, y = number("x"), number("y")
  if not (x and y) then
    print("✗ Pass cx and cy (world) or x and y (pixels)")
    return
  end
end

-- Keep the region inside the image.
local x2, y2 = math.min(x + w, spr.width), math.min(y + h, spr.height)
x, y = math.max(x, 0), math.max(y, 0)
w, h = x2 - x, y2 - y
if w <= 0 or h <= 0 then
  print("✗ The region is outside the image")
  return
end

spr:crop(x, y, w, h)
if scale ~= 1 then
  app.command.SpriteSize { ui = false, width = w * scale, height = h * scale, lockRatio = false, method = "nearest" }
end
spr:saveAs(out)
print(string.format("✓ %s: region %d,%d %dx%d, scale %d -> %dx%d", out, x, y, w, h, scale, spr.width, spr.height))
