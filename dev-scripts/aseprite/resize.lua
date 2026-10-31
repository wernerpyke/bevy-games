-- Resize the open sprite with nearest-neighbour scaling (keeps pixel art crisp) and save it in place.
--
-- Usage:
--   aseprite -b <file.aseprite> --script-param width=128 --script-param height=128 \
--     --script dev-scripts/aseprite/resize.lua
local spr = app.sprite
if not spr then
  print("✗ No sprite open")
  return
end

local width = tonumber(app.params["width"])
local height = tonumber(app.params["height"])
if not width or not height then
  print("✗ Pass width and height with --script-param")
  return
end

local old_w, old_h = spr.width, spr.height
app.command.SpriteSize { ui = false, width = width, height = height, lockRatio = false, method = "nearest" }
spr:saveAs(spr.filename)
print(string.format("✓ %s %dx%d -> %dx%d", app.fs.fileName(spr.filename), old_w, old_h, spr.width, spr.height))
