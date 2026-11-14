-- Make a new RGB .aseprite file with a plain bevelled button: two frames, tags Up (raised) and Down (pressed), and with
-- disabled=1 a third frame, tag Disabled (flat: no bevel, so it does not look pressable).
-- Colours are palette indexes of the open file (the game palette). Refuses to overwrite an existing file.
--
-- Usage (see dev-scripts/aseprite.sh; the input file is the palette file):
--   ./dev-scripts/aseprite.sh draw_button games/long-way/assets/art/game.aseprite \
--     out=games/long-way/assets/art/ui/weapon_button.aseprite width=118 height=56
--
--   out:            file to write (required)
--   width, height:  canvas size in pixels (required, at least 8 each)
--   outline:        index of the 1 pixel outline (default 36)
--   face:           index of the face (default 40)
--   light:          index of the lit edge: top and left when up, bottom and right when down (default 41)
--   shadow:         index of the shaded edge: bottom and right when up, top and left when down (default 36)
--   bevel:          width of the lit and shaded edges in pixels (default 2)
--   disabled:       1 adds the Disabled frame (default: no Disabled frame)
--   disabled_face:  index of the face and edges of the Disabled frame (default: the face)
-- The four corner pixels are left clear, so the outline looks rounded.
local palette_spr = app.sprite
local out = app.params["out"]
local width = tonumber(app.params["width"])
local height = tonumber(app.params["height"])
if not palette_spr or not out or not width or not height or width < 8 or height < 8 then
  print("✗ Usage: draw_button <palette.aseprite> out=<file> width=<n> height=<n> [outline= face= light= shadow= bevel= disabled=1 disabled_face=]")
  return
end
if app.fs.isFile(out) then
  print("✗ " .. out .. " exists, not overwriting")
  return
end

local palette = palette_spr.palettes[1]
local function colour(key, default)
  local index = tonumber(app.params[key]) or default
  if index >= #palette then
    error(string.format("%s=%d is outside the palette (%d colours)", key, index, #palette))
  end
  return palette:getColor(index), index
end
local outline, outline_i = colour("outline", 36)
local face, face_i = colour("face", 40)
local light, light_i = colour("light", 41)
local shadow, shadow_i = colour("shadow", 36)
local bevel = tonumber(app.params["bevel"]) or 2
local with_disabled = app.params["disabled"] == "1"
local disabled_face, disabled_face_i = colour("disabled_face", face_i)

-- One frame. `top_left` and `bottom_right` are the colours of the two bevel edges, `fill` the colour of the face.
local function draw(top_left, bottom_right, fill)
  local image = Image(width, height, ColorMode.RGB)
  image:clear(Color { r = 0, g = 0, b = 0, a = 0 })
  local w, h = width - 1, height - 1
  for y = 0, h do
    for x = 0, w do
      local on_edge = x == 0 or y == 0 or x == w or y == h
      local corner = (x == 0 or x == w) and (y == 0 or y == h)
      local c
      if corner then
        c = nil
      elseif on_edge then
        c = outline
      else
        -- Distance in from the outline to the left, top, right and bottom.
        local l, t, r, b = x - 1, y - 1, w - 1 - x, h - 1 - y
        if (t < bevel or l < bevel) and not (b < bevel and b < l) and not (r < bevel and r < t) then
          c = top_left
        elseif b < bevel or r < bevel then
          c = bottom_right
        else
          c = fill
        end
      end
      if c then
        image:drawPixel(x, y, c)
      end
    end
  end
  return image
end

local spr = Sprite(width, height, ColorMode.RGB)
spr:setPalette(palette)
local layer = spr.layers[1]
layer.name = "Button"
spr:newCel(layer, 1, draw(light, shadow, face), Point(0, 0))
spr:newEmptyFrame()
spr:newCel(layer, 2, draw(shadow, light, face), Point(0, 0))
if with_disabled then
  spr:newEmptyFrame()
  spr:newCel(layer, 3, draw(disabled_face, disabled_face, disabled_face), Point(0, 0))
end
-- Tags last, after every frame exists (art-pipeline.md, rule 9).
spr:newTag(1, 1).name = "Up"
spr:newTag(2, 2).name = "Down"
if with_disabled then
  spr:newTag(3, 3).name = "Disabled"
end
spr:saveAs(out)

local function hex(c)
  return string.format("#%02x%02x%02x", c.red, c.green, c.blue)
end
local disabled_text = with_disabled and string.format(", Disabled face %d (%s)", disabled_face_i, hex(disabled_face)) or ""
print(string.format("✓ %s: %dx%d, tags Up and Down%s, outline %d (%s), face %d (%s), light %d (%s), shadow %d (%s), bevel %d",
  out, width, height, disabled_text, outline_i, hex(outline), face_i, hex(face), light_i, hex(light), shadow_i, hex(shadow), bevel))
