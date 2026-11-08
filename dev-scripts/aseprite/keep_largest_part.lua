-- Clear everything in every cel of an RGB .aseprite file that is not joined to the largest visible part of that
-- cel (8 way connection). Used to remove loose marks such as blood splatters after their colours were cleared with
-- remove_colors.lua. Without apply=1 it only reports what it would clear and writes nothing.
--
-- Usage (see dev-scripts/aseprite.sh):
--   ./dev-scripts/aseprite.sh keep_largest_part <file.aseprite> [apply=1] [out=<file>]
--
--   apply: 1 saves the file (to out=, default the input file). Anything else is a dry run.
local spr = app.sprite
if not spr then
  print("✗ No sprite open")
  return
end
if spr.colorMode ~= ColorMode.RGB then
  print("✗ The sprite must be in RGB colour mode")
  return
end

local apply = app.params["apply"] == "1"
local out = app.params["out"] or spr.filename
local total = 0

for _, cel in ipairs(spr.cels) do
  local image = cel.image
  local w, h = image.width, image.height
  local label = {}
  local sizes = {}
  local next_label = 0
  for y = 0, h - 1 do
    for x = 0, w - 1 do
      if app.pixelColor.rgbaA(image:getPixel(x, y)) > 0 and not label[y * w + x] then
        next_label = next_label + 1
        local size = 0
        local stack = { y * w + x }
        label[y * w + x] = next_label
        while #stack > 0 do
          local p = table.remove(stack)
          size = size + 1
          local px, py = p % w, p // w
          for dy = -1, 1 do
            for dx = -1, 1 do
              local nx, ny = px + dx, py + dy
              if nx >= 0 and ny >= 0 and nx < w and ny < h then
                local n = ny * w + nx
                if not label[n] and app.pixelColor.rgbaA(image:getPixel(nx, ny)) > 0 then
                  label[n] = next_label
                  table.insert(stack, n)
                end
              end
            end
          end
        end
        sizes[next_label] = size
      end
    end
  end

  local keep, best = nil, 0
  for l, size in pairs(sizes) do
    if size > best then
      keep, best = l, size
    end
  end

  local cleared = 0
  for p, l in pairs(label) do
    if l ~= keep then
      cleared = cleared + 1
      if apply then
        image:putPixel(p % w, p // w, 0)
      end
    end
  end
  if cleared > 0 then
    print(string.format("  frame %d: %d px in %d loose parts", cel.frameNumber, cleared, next_label - 1))
  end
  total = total + cleared
end

if apply then
  spr:saveAs(out)
  print(string.format("✓ %s: cleared %d px, saved", app.fs.fileName(out), total))
else
  print(string.format("Dry run: would clear %d px. Pass apply=1 to save.", total))
end
