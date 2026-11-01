-- Build a new .aseprite file with one frame per PNG (all PNGs must be the same size), optionally
-- resize it with nearest-neighbour scaling, and tag each frame with the PNG file name.
--
-- Usage:
--   aseprite -b --script-param out=<file.aseprite> --script-param files=<a.png>,<b.png> \
--     [--script-param width=128 --script-param height=128] \
--     --script dev-scripts/aseprite/frames_from_pngs.lua
local out = app.params["out"]
local files = app.params["files"]
if not out or not files then
  print("✗ Pass out and files with --script-param")
  return
end

local paths = {}
for path in string.gmatch(files, "[^,]+") do
  table.insert(paths, path)
end

local first = Image { fromFile = paths[1] }
local spr = Sprite(first.width, first.height, ColorMode.RGB)
local layer = spr.layers[1]
for i, path in ipairs(paths) do
  local image = Image { fromFile = path }
  if image.width ~= spr.width or image.height ~= spr.height then
    print("✗ " .. path .. " is not the same size as the first PNG")
    return
  end
  local frame = i == 1 and spr.frames[1] or spr:newEmptyFrame()
  spr:newCel(layer, frame, image, Point(0, 0))
end

-- Tags are created after all frames exist. Adding a frame at the end would stretch an earlier tag.
for i, path in ipairs(paths) do
  local tag = spr:newTag(i, i)
  tag.name = app.fs.fileTitle(path)
end

local width = tonumber(app.params["width"])
local height = tonumber(app.params["height"])
if width and height then
  app.command.SpriteSize { ui = false, width = width, height = height, lockRatio = false, method = "nearest" }
end

spr:saveAs(out)
print(string.format("✓ %s %dx%d, %d frames", app.fs.fileName(out), spr.width, spr.height, #spr.frames))
