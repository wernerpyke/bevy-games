-- Build one .aseprite file from several sprite sheet PNGs (one row of equal frames each). Each sheet
-- becomes one tag. Frame durations come from the Aseprite JSON export that sits next to the sheet in
-- the json folder (same file name, .json). Each frame is moved onto a new canvas so the body is centred
-- and the feet are on a fixed row. A frame that would be cut off at a canvas edge is moved back inside.
--
-- Usage:
--   aseprite -b --script-param out=<file.aseprite> --script-param json=<json dir> \
--     --script-param sheets=<Tag>=<sheet.png>,<Tag>=<sheet.png>@<first>-<last> \
--     --script-param frame=96x64 --script-param canvas=64x64 \
--     --script-param from=34,47 --script-param to=32,60 \
--     --script dev-scripts/aseprite/tagged_sheets.lua
--   frame: size of one frame in the sheets. canvas: size of the new canvas.
--   from: x centre of the body and y of the feet in the source frame. to: the same in the new canvas.
--   @<first>-<last> after a sheet path takes only those frames (counted from 0, both included).
--   placeholders: optional comma list of tag names. Those tags get a red colour in Aseprite.
--   Sheet paths must not contain commas or @.
local function param(name)
  local value = app.params[name]
  if not value then
    print("✗ Pass " .. name .. " with --script-param")
  end
  return value
end

local function pair(text, separator)
  local a, b = string.match(text, "^(-?%d+)" .. separator .. "(-?%d+)$")
  return tonumber(a), tonumber(b)
end

local out, json_dir, sheets = param("out"), param("json"), param("sheets")
local frame_w, frame_h = pair(param("frame") or "", "x")
local canvas_w, canvas_h = pair(param("canvas") or "", "x")
local from_x, from_y = pair(param("from") or "", ",")
local to_x, to_y = pair(param("to") or "", ",")
if not (out and json_dir and sheets and frame_w and canvas_w and from_x and to_x) then
  return
end

local dx, dy = to_x - from_x, to_y - from_y
local spr = Sprite(canvas_w, canvas_h, ColorMode.RGB)
local layer = spr.layers[1]
layer.name = "Body"

local ranges = {}
local first = true
for entry in string.gmatch(sheets, "[^,]+") do
  local tag_name, path = string.match(entry, "^([^=]+)=(.+)$")
  local first_index, last_index
  local range_path, range_from, range_to = string.match(path, "^(.-)@(%d+)-(%d+)$")
  if range_path then
    path, first_index, last_index = range_path, tonumber(range_from), tonumber(range_to)
  end
  local json = json.decode(io.open(json_dir .. "/" .. app.fs.fileTitle(path) .. ".json"):read("a"))
  local sheet = Image { fromFile = path }
  local count = #json.frames
  if sheet.width ~= count * frame_w or sheet.height ~= frame_h then
    print(string.format("✗ %s is %dx%d, expected %d frames of %dx%d", path, sheet.width, sheet.height, count, frame_w, frame_h))
    return
  end

  local from_frame = #spr.frames + (first and 0 or 1)
  for i = (first_index or 0) + 1, (last_index or count - 1) + 1 do
    local source = Image(sheet, Rectangle((i - 1) * frame_w, 0, frame_w, frame_h))
    local target = Image(canvas_w, canvas_h, ColorMode.RGB)
    local x = dx
    local bounds = source:shrinkBounds()
    if not source:isEmpty() then
      -- Keep the whole visible part inside the canvas.
      if bounds.x + x < 0 then
        x = -bounds.x
      elseif bounds.x + bounds.width + x > canvas_w then
        x = canvas_w - bounds.x - bounds.width
      end
      if x ~= dx then
        print(string.format("  %s frame %d moved %d px to stay inside the canvas", tag_name, i - 1, x - dx))
      end
    end
    target:drawImage(source, Point(x, dy))

    local frame = first and spr.frames[1] or spr:newEmptyFrame()
    first = false
    frame.duration = json.frames[i].duration / 1000
    spr:newCel(layer, frame, target, Point(0, 0))
  end
  table.insert(ranges, { tag_name, from_frame, #spr.frames })
end

local placeholder_tags = {}
for name in string.gmatch(app.params["placeholders"] or "", "[^,]+") do
  placeholder_tags[name] = true
end

-- Tags are created after all frames exist. Adding a frame at the end would stretch an earlier tag.
for _, range in ipairs(ranges) do
  local tag = spr:newTag(range[2], range[3])
  tag.name = range[1]
  if placeholder_tags[range[1]] then
    tag.color = Color { r = 225, g = 55, b = 55 }
  end
end

spr:saveAs(out)
print(string.format("✓ %s %dx%d, %d frames, %d tags", app.fs.fileName(out), spr.width, spr.height, #spr.frames, #ranges))
