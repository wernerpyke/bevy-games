-- Set the frame duration of the frames in some tags of the open .aseprite file. Without apply=1 it only reports
-- what it would change and writes nothing. Use `check` to read durations.
--
-- Usage (see dev-scripts/aseprite.sh):
--   aseprite -b <file.aseprite> --script-param tags=marker,marker_red --script-param ms=250 \
--     [--script-param apply=1] [--script-param out=<file.aseprite>] --script dev-scripts/aseprite/set_durations.lua
--
--   tags:  comma separated tag names, or "all" for every frame of the file
--   ms:    new duration in milliseconds, for every frame of those tags
--   apply: 1 saves the file (to out=, default the input file). Anything else is a dry run.
-- A frame in two named tags gets the same value from either. A tag name that does not exist is an error.
local spr = app.sprite
if not spr then
  print("✗ No sprite open")
  return
end

local ms = tonumber(app.params["ms"])
local tag_param = app.params["tags"]
if not ms or ms <= 0 or not tag_param then
  print("✗ Usage: set_durations <file> tags=<a,b|all> ms=<milliseconds> [apply=1] [out=<file>]")
  return
end

local frames = {}
if tag_param == "all" then
  for n = 1, #spr.frames do
    frames[n] = true
  end
else
  for name in tag_param:gmatch("[^,]+") do
    local found = false
    for _, tag in ipairs(spr.tags) do
      if tag.name == name then
        found = true
        for n = tag.fromFrame.frameNumber, tag.toFrame.frameNumber do
          frames[n] = true
        end
      end
    end
    if not found then
      print("✗ No tag named " .. name)
      return
    end
  end
end

local changed = 0
for n = 1, #spr.frames do
  if frames[n] then
    local before = math.floor(spr.frames[n].duration * 1000 + 0.5)
    if before ~= ms then
      print(string.format("  frame %d: %d ms -> %d ms", n, before, ms))
      spr.frames[n].duration = ms / 1000
      changed = changed + 1
    end
  end
end

if changed == 0 then
  print(string.format("✓ Nothing to change, every selected frame is already %d ms", ms))
  return
end
if app.params["apply"] ~= "1" then
  print(string.format("Dry run: %d frames would change. Add apply=1 to save.", changed))
  return
end
local path = app.params["out"] or spr.filename
spr:saveAs(path)
print(string.format("✓ Set %d frames to %d ms, saved %s", changed, ms, path))
