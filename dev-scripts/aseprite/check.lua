-- Report on the open .aseprite file: canvas, colour mode, layers, tags with frame ranges and
-- durations, palette check, and the bounds of the visible pixels in every frame. Read only.
--
-- Usage (see dev-scripts/aseprite.sh):
--   aseprite -b <file.aseprite> --script-param palette=<diodrama_palette.aseprite> \
--     --script dev-scripts/aseprite/check.lua
--
--   palette: optional .aseprite file. Every visible pixel colour must be in its palette.
--   edge:    optional, default 0. A frame is reported as "cut off" when pixels touch the
--            canvas edge, or lie within this many pixels of it.
-- Bounds use 0-based pixel coordinates, as in the game docs: `x a-b, y a-b` includes both ends.
-- `feet` is the lowest visible row. Frames are numbered from 1, as in Aseprite.
local spr = app.sprite
if not spr then
  print("✗ No sprite open")
  return
end

local edge = tonumber(app.params["edge"]) or 0
local mode_names = { [ColorMode.RGB] = "RGB", [ColorMode.GRAYSCALE] = "grayscale", [ColorMode.INDEXED] = "indexed" }
print(string.format("File: %s", app.fs.fileName(spr.filename)))
print(string.format("Canvas: %dx%d, colour mode %s, %d frames", spr.width, spr.height, mode_names[spr.colorMode] or "?", #spr.frames))

local function layer_lines(layers, indent)
  for _, layer in ipairs(layers) do
    print(string.format("%sLayer %s (%s)", indent, layer.name, layer.isVisible and "visible" or "hidden"))
    if layer.isGroup then
      layer_lines(layer.layers, indent .. "  ")
    end
  end
end
layer_lines(spr.layers, "  ")

local direction_names = {
  [AniDir.FORWARD] = "forward",
  [AniDir.REVERSE] = "reverse",
  [AniDir.PING_PONG] = "ping-pong",
  [AniDir.PING_PONG_REVERSE] = "ping-pong reverse",
}
print("Tags:")
if #spr.tags == 0 then
  print("  none")
end
for _, tag in ipairs(spr.tags) do
  local from, to = tag.fromFrame.frameNumber, tag.toFrame.frameNumber
  local durations = {}
  for n = from, to do
    durations[#durations + 1] = math.floor(spr.frames[n].duration * 1000 + 0.5)
  end
  print(string.format("  %s: frames %d-%d (%d), %s ms, %s", tag.name, from, to, to - from + 1, table.concat(durations, " "), direction_names[tag.aniDir] or "?"))
end

-- Colours of the palette file, as "rrggbb" keys.
local allowed
local palette_path = app.params["palette"]
if palette_path then
  local palette_sprite = Sprite { fromFile = palette_path }
  if not palette_sprite then
    print("✗ Cannot open palette " .. palette_path)
    return
  end
  allowed = {}
  local palette = palette_sprite.palettes[1]
  for i = 0, #palette - 1 do
    local c = palette:getColor(i)
    allowed[string.format("%02x%02x%02x", c.red, c.green, c.blue)] = true
  end
  palette_sprite:close()
  app.sprite = spr
end

local function rgba_of(value, palette)
  if spr.colorMode == ColorMode.RGB then
    return app.pixelColor.rgbaR(value), app.pixelColor.rgbaG(value), app.pixelColor.rgbaB(value), app.pixelColor.rgbaA(value)
  elseif spr.colorMode == ColorMode.GRAYSCALE then
    local v = app.pixelColor.grayaV(value)
    return v, v, v, app.pixelColor.grayaA(value)
  else
    local c = palette:getColor(value)
    return c.red, c.green, c.blue, c.alpha
  end
end

print("Frames (visible pixels):")
local colours = {}
local unknown = {}
local palette = spr.palettes[1]
local cut_off = {}
for n = 1, #spr.frames do
  local image = Image(spr.spec)
  image:clear()
  image:drawSprite(spr, n)
  local min_x, min_y, max_x, max_y = spr.width, spr.height, -1, -1
  for pixel in image:pixels() do
    local r, g, b, a = rgba_of(pixel(), palette)
    if a > 0 then
      local x, y = pixel.x, pixel.y
      min_x, min_y = math.min(min_x, x), math.min(min_y, y)
      max_x, max_y = math.max(max_x, x), math.max(max_y, y)
      local key = string.format("%02x%02x%02x", r, g, b)
      colours[key] = true
      if allowed and not allowed[key] then
        unknown[key] = (unknown[key] or 0) + 1
      end
    end
  end
  if max_x < 0 then
    print(string.format("  %3d: empty", n))
  else
    local touches = min_x <= edge or min_y <= edge or max_x >= spr.width - 1 - edge or max_y >= spr.height - 1 - edge
    local width, height = max_x - min_x + 1, max_y - min_y + 1
    print(string.format("  %3d: x %d-%d, y %d-%d, body %dx%d, centre x %.1f, feet y %d%s", n, min_x, max_x, min_y, max_y, width, height, (min_x + max_x) / 2, max_y, touches and "  CUT OFF" or ""))
    if touches then
      cut_off[#cut_off + 1] = n
    end
  end
end

local count = 0
for _ in pairs(colours) do
  count = count + 1
end
print(string.format("Visible colours: %d", count))
if allowed then
  local bad = {}
  for key, pixels in pairs(unknown) do
    bad[#bad + 1] = string.format("#%s (%d px)", key, pixels)
  end
  table.sort(bad)
  if #bad == 0 then
    print("✓ Every visible colour is in the palette")
  else
    print("✗ Colours not in the palette: " .. table.concat(bad, ", "))
  end
end
if #cut_off > 0 then
  print("✗ Frames that touch the canvas edge: " .. table.concat(cut_off, ", "))
end
