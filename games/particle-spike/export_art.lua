-- Export the particle sprites of fireball_placeholder.aseprite as PNG sheets for Enoki.
-- Each tag becomes one sheet `<tag>.png`: the frames of the tag side by side, every frame cropped to the union
-- of the tag's visible pixels (so all cells have one size). `<tag>_last.png` is the last frame alone.
-- Run from the workspace root:
--   aseprite -b games/particle-spike/assets/art/fireball_placeholder.aseprite \
--     --script-param out=games/particle-spike/assets/generated --script games/particle-spike/export_art.lua
local spr = app.sprite
local out = app.params["out"]
local names = { blob = true, wall_fire = true, wall_ice = true }

for _, tag in ipairs(spr.tags) do
  if names[tag.name] then
    local first, last = tag.fromFrame.frameNumber, tag.toFrame.frameNumber
    local frames, bounds = {}, nil
    for f = first, last do
      local img = Image(spr.spec)
      img:drawSprite(spr, f)
      frames[#frames + 1] = img
      local r = img:shrinkBounds()
      bounds = bounds and bounds:union(r) or r
    end
    local w, h = bounds.width, bounds.height
    -- Enoki draws every particle as a square quad, so a sheet with tall cells would be stretched. The wall sprites are
    -- tall, so their cells are made square, with the sprite centred in the cell.
    local cw, ch = w, h
    if string.sub(tag.name, 1, 4) == "wall" then cw, ch = math.max(w, h), math.max(w, h) end
    local function cell(img)
      local c = Image(cw, ch, ColorMode.RGB)
      c:drawImage(Image(img, bounds), Point((cw - w) // 2, (ch - h) // 2))
      return c
    end
    local sheet = Image(cw * #frames, ch, ColorMode.RGB)
    for i, img in ipairs(frames) do
      sheet:drawImage(cell(img), Point((i - 1) * cw, 0))
    end
    sheet:saveAs(out .. "/" .. tag.name .. ".png")
    cell(frames[#frames]):saveAs(out .. "/" .. tag.name .. "_last.png")
    print(string.format("%s: %d frames, cell %dx%d", tag.name, #frames, cw, ch))
  end
end
