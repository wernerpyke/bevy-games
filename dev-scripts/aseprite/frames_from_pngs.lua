-- Build a new .aseprite file with one frame per PNG, optionally resize every PNG to width x height with
-- nearest-neighbour scaling, and tag each frame with the PNG file name. Without width and height all PNGs
-- must be the same size.
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

local width = tonumber(app.params["width"])
local height = tonumber(app.params["height"])
local resize = width and height

-- One PNG as an image, resized with nearest-neighbour scaling when a target size is given.
local function load_image(path)
  local image = Image { fromFile = path }
  if not resize or (image.width == width and image.height == height) then
    return image
  end
  local tmp = Sprite(image.width, image.height, ColorMode.RGB)
  tmp:newCel(tmp.layers[1], tmp.frames[1], image, Point(0, 0))
  app.command.SpriteSize { ui = false, width = width, height = height, lockRatio = false, method = "nearest" }
  local resized = Image(tmp.cels[1].image)
  tmp:close()
  return resized
end

local first = load_image(paths[1])
local spr = Sprite(first.width, first.height, ColorMode.RGB)
local layer = spr.layers[1]
for i, path in ipairs(paths) do
  local image = load_image(path)
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

spr:saveAs(out)
print(string.format("✓ %s %dx%d, %d frames", app.fs.fileName(out), spr.width, spr.height, #spr.frames))
