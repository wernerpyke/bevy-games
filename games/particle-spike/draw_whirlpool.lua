-- Draw the placeholder art of the whirlpool ring (variant C): a band of water with spiral arms that turn clockwise.
-- One tag `ring`, FRAMES frames that loop without a jump: the arm pattern turns by one arm spacing over the loop.
-- 256x256 canvas, centre of the ring at the canvas centre, band from R0 to R1. The frame time is set here (80 ms).
-- Own colours (not the game palette): placeholder. Run from the workspace root, it does not overwrite:
--   aseprite -b --script-param out=games/particle-spike/assets/art/whirlpool_placeholder.aseprite \
--     --script games/particle-spike/draw_whirlpool.lua
local out = app.params["out"]
if not out or app.fs.isFile(out) then
  print("x give out=<new file>, existing files are not overwritten")
  return
end

local SIZE, FRAMES = 256, 12
local R0, R1 = 64, 114 -- the band; the hurt band of the game (70 to 110) sits inside it
local ARMS, TWIST = 4, 2.6 -- spiral arms, and how far (radians) an arm turns from the inner to the outer edge
local FRAME_SECS = 0.08

local function c(hex)
  return Color { r = tonumber(hex:sub(1, 2), 16), g = tonumber(hex:sub(3, 4), 16), b = tonumber(hex:sub(5, 6), 16), a = 255 }
end
local FOAM, LIGHT, MID, DARK = c("eaf8ff"), c("9bd8ff"), c("4aa0e8"), c("2b6cc0")

local function hash(x, y) return ((x * 73856093) ~ (y * 19349663)) % 100 / 100 end

local function ring(frame)
  local img = Image(SIZE, SIZE, ColorMode.RGB)
  local shift = 2 * math.pi * frame / FRAMES
  for y = 0, SIZE - 1 do
    for x = 0, SIZE - 1 do
      local dx, dy = x + 0.5 - SIZE / 2, y + 0.5 - SIZE / 2
      local r = math.sqrt(dx * dx + dy * dy)
      if r >= R0 and r <= R1 then
        local u = (r - R0) / (R1 - R0)
        local theta = math.atan(dy, dx) -- y points down, so a growing angle turns clockwise on screen
        local arm = ((math.cos(ARMS * theta - TWIST * u - shift) + 1) / 2) ^ 3 -- thin crests
        arm = arm + (hash(x, y) - 0.5) * 0.12
        -- a solid band of water: dark at both edges, mid in between, with the crests on top
        local colour = MID
        if r < R0 + 2.5 or r > R1 - 2.5 then colour = DARK end
        if arm > 0.88 then colour = FOAM
        elseif arm > 0.55 then colour = LIGHT
        elseif arm > 0.3 and colour == MID then colour = c("6fb8f0") end
        img:putPixel(x, y, colour)
      end
    end
  end
  return img
end

local spr = Sprite(SIZE, SIZE, ColorMode.RGB)
spr.layers[1].name = "Water"
for f = 1, FRAMES do
  if f > 1 then spr:newEmptyFrame() end
  spr:newCel(spr.layers[1], f, ring(f - 1), Point(0, 0))
  spr.frames[f].duration = FRAME_SECS
end
-- The tag is made last: a tag that ends on the last frame stretches when a frame is added after it.
spr:newTag(1, FRAMES).name = "ring"
spr:saveAs(out)
print("ok " .. out .. ": " .. FRAMES .. " frames")
