-- Draw the placeholder art of the ice wall: 3 ice cubes of different heights, each growing out of the ground.
-- Tags `cube_s`, `cube_m`, `cube_t` (short, medium, tall), 4 frames each: three growth frames (20%, 45%, 75% of the
-- height) and the full cube. 64x64 canvas, the ground line is the bottom row, the cube is centred sideways.
-- Own colours (not the game palette): placeholder. Run from the workspace root, it does not overwrite:
--   aseprite -b --script-param out=games/particle-spike/assets/art/ice_wall_placeholder.aseprite \
--     --script games/particle-spike/draw_ice_cubes.lua
local out = app.params["out"]
if not out or app.fs.isFile(out) then
  print("x give out=<new file>, existing files are not overwritten")
  return
end

local SIZE, W, D = 64, 22, 7 -- canvas, front face width, depth skew (45 degrees)
local HEIGHTS = { cube_s = 18, cube_m = 32, cube_t = 46 }
local FRACTIONS = { 0.2, 0.45, 0.75, 1.0 }

local function c(hex)
  return Color { r = tonumber(hex:sub(1, 2), 16), g = tonumber(hex:sub(3, 4), 16), b = tonumber(hex:sub(5, 6), 16), a = 255 }
end
local TOP, FRONT, FRONT_LOW = c("eafaff"), c("9fd8ff"), c("86c4f5")
local RIGHT, RIGHT_LOW = c("4f9fe6"), c("3a82cc")
local EDGE, GLINT, OUTLINE = c("ffffff"), c("d4f1ff"), c("2d5b9e")

-- Deterministic scatter for the ice texture.
local function hash(x, y, seed) return ((x * 73856093) ~ (y * 19349663) ~ (seed * 83492791)) % 97 end

local function cube(h, seed)
  local img = Image(SIZE, SIZE, ColorMode.RGB)
  local x0 = (SIZE - (W + D)) // 2
  local x1 = x0 + W - 1
  local y1 = SIZE - 1
  local ytop = y1 - h + 1 -- first row of the front face
  local mask = {}
  local function put(x, y, colour)
    if x >= 0 and x < SIZE and y >= 0 and y < SIZE then
      img:putPixel(x, y, colour)
      mask[y * SIZE + x] = true
    end
  end
  -- front face: lighter at the top, a little darker in the lower quarter
  for y = ytop, y1 do
    for x = x0, x1 do
      local low = y > y1 - math.max(2, h // 4)
      local col = low and FRONT_LOW or FRONT
      -- a diagonal glint and a few specks
      local d = (x - x0) + (y - ytop)
      if (d % 17 == 5 or d % 17 == 6) and y < y1 - 2 and x > x0 + 1 then col = GLINT end
      if hash(x, y, seed) < 3 and not low then col = GLINT end
      put(x, y, col)
    end
  end
  -- top face: rows above the front face, each shifted right by its depth
  for i = 1, D do
    for x = x0 + i, x1 + i do put(x, ytop - i, i == D and EDGE or TOP) end
  end
  -- right face: columns right of the front face, each shifted up by its depth
  for j = 1, D do
    for y = ytop - j + 1, y1 - j do
      put(x1 + j, y, (y > y1 - j - math.max(2, h // 4)) and RIGHT_LOW or RIGHT)
    end
  end
  -- bright edges where the faces meet
  for x = x0, x1 do put(x, ytop, EDGE) end
  for y = ytop, y1 do put(x1, y, GLINT) end
  -- dark outline on the silhouette, so overlapping cubes stay readable
  local outline = {}
  for y = 0, SIZE - 1 do
    for x = 0, SIZE - 1 do
      if not mask[y * SIZE + x] then
        for _, n in ipairs { { 1, 0 }, { -1, 0 }, { 0, 1 }, { 0, -1 } } do
          if mask[(y + n[2]) * SIZE + x + n[1]] then outline[#outline + 1] = { x, y } break end
        end
      end
    end
  end
  for _, p in ipairs(outline) do img:putPixel(p[1], p[2], OUTLINE) end
  return img
end

local spr = Sprite(SIZE, SIZE, ColorMode.RGB)
spr.layers[1].name = "Ice"
local frame = 0
local tag_ranges = {}
for _, name in ipairs { "cube_s", "cube_m", "cube_t" } do
  local first = frame + 1
  for k, f in ipairs(FRACTIONS) do
    frame = frame + 1
    if frame > 1 then spr:newEmptyFrame() end
    local h = math.max(3, math.floor(HEIGHTS[name] * f + 0.5))
    spr:newCel(spr.layers[1], frame, cube(h, frame), Point(0, 0))
  end
  tag_ranges[#tag_ranges + 1] = { name, first, frame }
end
-- Tags are made last: a tag that ends on the last frame stretches when a frame is added after it.
for _, t in ipairs(tag_ranges) do spr:newTag(t[2], t[3]).name = t[1] end
spr:saveAs(out)
print("ok " .. out .. ": " .. frame .. " frames")
