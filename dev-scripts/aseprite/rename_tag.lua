-- Rename one tag of the open .aseprite file. Without apply=1 it only reports what it would change and writes nothing.
--
-- Usage (see dev-scripts/aseprite.sh):
--   aseprite -b <file.aseprite> --script-param from=whirlpool --script-param to=tornado \
--     [--script-param apply=1] [--script-param out=<file.aseprite>] --script dev-scripts/aseprite/rename_tag.lua
--
--   from:  the tag to rename. It must exist once.
--   to:    the new name. No other tag may have it.
--   apply: 1 saves the file (to out=, default the input file). Anything else is a dry run.
local spr = app.sprite
if not spr then
  print("✗ No sprite open")
  return
end

local from = app.params["from"]
local to = app.params["to"]
if not from or not to or from == "" or to == "" then
  print("✗ Usage: rename_tag <file> from=<tag> to=<tag> [apply=1] [out=<file>]")
  return
end

local found = nil
for _, tag in ipairs(spr.tags) do
  if tag.name == to then
    print("✗ A tag named " .. to .. " exists already")
    return
  end
  if tag.name == from then
    if found then
      print("✗ More than one tag is named " .. from)
      return
    end
    found = tag
  end
end
if not found then
  print("✗ No tag named " .. from)
  return
end

print(string.format("%s -> %s (frames %d-%d)", from, to, found.fromFrame.frameNumber, found.toFrame.frameNumber))
if app.params["apply"] ~= "1" then
  print("Dry run: nothing written. Add apply=1 to save.")
  return
end
found.name = to
local out = app.params["out"] or spr.filename
spr:saveAs(out)
print("✓ Saved " .. out)
