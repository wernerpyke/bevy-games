-- Export chosen frames or a tag of the open .aseprite file as one scaled grid PNG, to look at an
-- animation without opening Aseprite. Visible layers only, as in the game. The file is not changed.
--
-- Usage (see dev-scripts/aseprite.sh):
--   aseprite -b <file.aseprite> --script-param out=<grid.png> --script-param tag=Run \
--     --script-param scale=2 --script-param cols=8 --script dev-scripts/aseprite/frame_grid.lua
--
--   out:    result PNG.
--   tag:    a tag name. Or frames=<first>-<last> (1-based, as in Aseprite). Default: all frames.
--   cols:   frames per row, default 8.
--   scale:  whole-number zoom, default 1.
--   gap:    pixels between frames before scaling, default 2.
-- A frame is drawn on a dark grey cell, so transparent pixels can be told from black ones.
local spr = app.sprite
if not spr then
  print("✗ No sprite open")
  return
end

local out = app.params["out"]
if not out then
  print("✗ Pass out with --script-param")
  return
end
local cols = tonumber(app.params["cols"]) or 8
local scale = tonumber(app.params["scale"]) or 1
local gap = tonumber(app.params["gap"]) or 2

local first, last = 1, #spr.frames
local tag_name = app.params["tag"]
if tag_name then
  local found
  for _, tag in ipairs(spr.tags) do
    if tag.name == tag_name then
      found = tag
    end
  end
  if not found then
    print("✗ No tag called " .. tag_name)
    return
  end
  first, last = found.fromFrame.frameNumber, found.toFrame.frameNumber
elseif app.params["frames"] then
  local a, b = string.match(app.params["frames"], "^(%d+)-(%d+)$")
  if not a then
    print("✗ frames must look like 3-10")
    return
  end
  first, last = tonumber(a), math.min(tonumber(b), #spr.frames)
end

local count = last - first + 1
if count < 1 then
  print("✗ No frames selected")
  return
end
cols = math.min(cols, count)
local rows = math.ceil(count / cols)
local width = cols * spr.width + (cols + 1) * gap
local height = rows * spr.height + (rows + 1) * gap

local grid = Image(width, height, ColorMode.RGB)
grid:clear(Color { r = 40, g = 40, b = 40, a = 255 })
local cell_colour = Color { r = 70, g = 70, b = 70, a = 255 }
for n = 0, count - 1 do
  local px = gap + (n % cols) * (spr.width + gap)
  local py = gap + math.floor(n / cols) * (spr.height + gap)
  local cell = Image(spr.width, spr.height, ColorMode.RGB)
  cell:clear(cell_colour)
  cell:drawSprite(spr, first + n, Point(0, 0))
  grid:drawImage(cell, Point(px, py))
end

local result = Sprite(width, height, ColorMode.RGB)
result.cels[1].image = grid
if scale ~= 1 then
  app.command.SpriteSize { ui = false, width = width * scale, height = height * scale, lockRatio = false, method = "nearest" }
end
result:saveAs(out)
print(string.format("✓ %s: frames %d-%d (%d), %d columns, %dx%d", out, first, last, count, cols, result.width, result.height))
