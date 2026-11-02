-- Build a new .aseprite file with one frame per PNG. The PNGs may have different sizes. The canvas is the largest
-- width and the largest height, and each image sits at the bottom centre of its frame, so feet and tree bases of
-- every sprite share one line. Each frame gets a tag named <prefix><PNG file name without extension>.
--
-- Usage:
--   aseprite -b --script-param out=<file.aseprite> --script-param files=<a.png>,<b.png> \
--     --script-param prefix=char_ --script dev-scripts/aseprite/pack_pngs.lua
--
-- It prints one "frame <n> <tag> <width>x<height> at <x>,<y>" line per image, for the art catalogue.
local out = app.params["out"]
local files = app.params["files"]
local prefix = app.params["prefix"] or ""
if not out or not files then
  print("✗ Pass out and files with --script-param")
  return
end

local paths = {}
for path in string.gmatch(files, "[^,]+") do
  table.insert(paths, path)
end

local images = {}
local width, height = 0, 0
for i, path in ipairs(paths) do
  images[i] = Image { fromFile = path }
  width = math.max(width, images[i].width)
  height = math.max(height, images[i].height)
end

local spr = Sprite(width, height, ColorMode.RGB)
local layer = spr.layers[1]
local lines = {}
for i, image in ipairs(images) do
  local x = (width - image.width) // 2
  local y = height - image.height
  local frame = i == 1 and spr.frames[1] or spr:newEmptyFrame()
  spr:newCel(layer, frame, image, Point(x, y))
  table.insert(lines, string.format("frame %d %s%s %dx%d at %d,%d", i, prefix, app.fs.fileTitle(paths[i]),
    image.width, image.height, x, y))
end

-- Tags are created after all frames exist. Adding a frame at the end would stretch an earlier tag.
for i, path in ipairs(paths) do
  local tag = spr:newTag(i, i)
  tag.name = prefix .. app.fs.fileTitle(path)
end

spr:saveAs(out)
for _, line in ipairs(lines) do
  print(line)
end
print(string.format("✓ %s %dx%d, %d frames", app.fs.fileName(out), spr.width, spr.height, #spr.frames))
