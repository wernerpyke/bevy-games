-- Build one .aseprite file from several PNG sheets whose frames are stacked in one column (top to bottom) or in
-- one row (left to right). Each sheet becomes one tag. Frames are copied unchanged onto a canvas of the frame
-- size.
--
-- Usage:
--   aseprite -b --script-param out=<file.aseprite> --script-param frame=90x90 \
--     --script-param sheets=<Tag>=<sheet.png>,<Tag>=<sheet.png> [--script-param duration=100] \
--     --script dev-scripts/aseprite/tagged_stacks.lua
--   direction: vertical, horizontal or auto (default). Auto reads the sheet size: one frame wide is a column,
--     one frame high is a row. A sheet that is exactly one frame has one frame.
--   duration: ms per frame, default 100. Sheet paths must not contain commas.
local out, sheets = app.params["out"], app.params["sheets"]
local frame_w, frame_h = string.match(app.params["frame"] or "", "^(%d+)x(%d+)$")
frame_w, frame_h = tonumber(frame_w), tonumber(frame_h)
local duration = (tonumber(app.params["duration"]) or 100) / 1000
if not (out and sheets and frame_w) then
  print("✗ Pass out, frame and sheets with --script-param")
  return
end

local spr = Sprite(frame_w, frame_h, ColorMode.RGB)
spr.layers[1].name = "Body"
local ranges = {}
for entry in string.gmatch(sheets, "[^,]+") do
  local tag_name, path = string.match(entry, "^([^=]+)=(.+)$")
  local sheet = Image { fromFile = path }
  local direction = app.params["direction"] or "auto"
  local is_column = sheet.width == frame_w and sheet.height % frame_h == 0
  local is_row = sheet.height == frame_h and sheet.width % frame_w == 0
  if direction == "auto" then
    direction = is_column and "vertical" or "horizontal"
  end
  local vertical = direction == "vertical"
  if not (vertical and is_column or not vertical and is_row) then
    print(string.format("✗ %s is %dx%d, not a column or a row of %dx%d frames (direction %s)", path, sheet.width, sheet.height, frame_w, frame_h, direction))
    return
  end
  local first = #spr.frames
  local count = vertical and sheet.height // frame_h or sheet.width // frame_w
  for i = 0, count - 1 do
    local frame
    if #ranges == 0 and i == 0 then
      frame = spr.frames[1]
    else
      frame = spr:newEmptyFrame()
    end
    frame.duration = duration
    local area = vertical and Rectangle(0, i * frame_h, frame_w, frame_h) or Rectangle(i * frame_w, 0, frame_w, frame_h)
    spr:newCel(spr.layers[1], frame, Image(sheet, area), Point(0, 0))
  end
  local from = (#ranges == 0) and 1 or first + 1
  table.insert(ranges, { tag_name, from, from + count - 1 })
end

-- Tags are created after all frames exist so an earlier tag is not stretched.
for _, r in ipairs(ranges) do
  spr:newTag(r[2], r[3]).name = r[1]
end

spr:saveAs(out)
print(string.format("✓ %s %dx%d, %d frames", app.fs.fileName(out), spr.width, spr.height, #spr.frames))
