# Tile sheet catalogue

## Summary

A bought tile sheet is hard for a person to read: a 1792x512 PNG has no coordinates, and a folder of numbered single tiles hides which tiles form one object. A catalogue is an `.aseprite` copy of the sheet with a column and row ruler and a grid. A person finds a sprite by looking at the catalogue and names it as `c<col> r<row>` plus its size in tiles. Claude then cuts that block out of the original sheet.

## Status

Ready. `dev-scripts/aseprite/catalogue.lua` is used through `./dev-scripts/aseprite.sh catalogue`. Decision: `adr-a7`. It has no game code and no tests. Check the result by looking at it.

## Terms

- **Sheet.** One PNG that holds many tiles or objects on a regular grid.
- **Catalogue.** The `.aseprite` file made from a sheet: backdrop, the sheet at 1:1, a grid and rulers, in four layers.
- **Column and row.** A tile position counted from 0 at the top left. Tile `(c, r)` starts at pixel `(c * tile, r * tile)` in the sheet. The catalogue has a margin on the top and left, so the same tile starts at `(c * tile + 20, r * tile + 20)` in the catalogue file. Always cut from the original sheet, not from the catalogue.
- **Block.** An object that covers several tiles, such as a 3x5 pagoda. It is named by its top left tile and its size in tiles.

## Parameters the game supplies

| Parameter | Meaning | Long Way (Zen Garden) |
|---|---|---|
| `tile` (or `tilew` and `tileh`) | Size of one tile in pixels | 32 |
| Sheet | The PNG to catalogue | `input/zen-garden/Non-RPGMaker_Tilemap/Main_Tilemap.png` |
| Catalogue file | Where the result goes | `input/zen-garden/Main_Tilemap_catalogue.aseprite` |

## Specification

### Making a catalogue

1. `./dev-scripts/aseprite.sh catalogue <sheet.png> tile=<px> out=<file.aseprite>`. Optional: `backdrop=RRGGBB` (default `e8e0d0`).
2. The sheet is drawn at 1:1. It is never scaled up (`adr-a7`). People zoom in Aseprite.
3. Layers, bottom to top: `Backdrop` (a solid colour, so transparent areas can be seen), `Sheet`, `Grid`, `Rulers`. Hide a layer to see the art clean.
4. The grid has a thin line on every tile edge and a red line every 8 tiles. The red lines make it quick to find a region in a large sheet.
5. The rulers sit in a margin outside the art, so no label covers a pixel. Column numbers are on top and row numbers on the left, each centred on its tile.

### Making any tile sheet human readable

These rules came from the Zen Garden pack and apply to any pack.

1. **Use the sheet, not the pre-cut tiles.** A folder of numbered single tiles (`GK_TAS_A_137.png`) says nothing about what is next to what, what belongs together, or which colour variant it is. The sheet shows the objects whole. Pre-cut tiles are only useful when each file has a real name.
2. **Prefer the engine-neutral sheet.** RPG Maker layouts (A1, A2, A5, B, C, D) pack autotiles and objects by engine rules. A plain tilemap shows objects as drawn.
3. **Mark coordinates with a ruler, not with labels on top of the art.** A margin ruler never hides pixels. Count from 0 so the column number equals the pixel offset divided by the tile size.
4. **Never scale the source PNG.** Scale a single sprite in Aseprite when it is extracted. Scaling the whole sheet adds a second copy that can drift from the original and makes pixel positions harder to work out.
5. **Show transparency with a backdrop layer.** A viewer shows alpha 0 as white. A coloured backdrop on its own layer makes clear what is transparent, and it never changes the sheet. Check the real alpha of the background before you "fix" it (`app.pixelColor.rgbaA`). Zen Garden was already transparent.
6. **Add a stronger line at regular steps** (every 8 tiles) so a person can count from a landmark.
7. **Keep the original, the catalogue and the extracted sprites apart.** Original sheet and catalogue stay in `input/`. Extracted sprites go in the game's `assets/art/`. Each extracted sprite is recorded in the game's art pipeline file with source, position and size.
8. **Name a block by top left tile and size** (`c32 r0, 3x5`). If only the top left is known, find the edges from the pixels. The block's bounds are where the tiles stop being opaque.
9. **Cut from the original sheet.** Keep the pixels exact: no resampling, no colour change at cut time. Resizing or palette work is a separate step.
10. **Do not create a catalogue for named files.** Packs that already ship named single files (for example `Koi1.png`, `Fountaintop2.png`) are read directly.

### Cutting a sprite

`./dev-scripts/aseprite.sh extract_block <original sheet.png> col=<c> row=<r> cols=<w> rows=<h> tile=<px> scale=<n> anchor=<mode> out=<file.aseprite>`. The input is the original sheet, not the catalogue.

1. The block (`cols` x `rows` tiles) is the **cell space** the sprite is anchored in.
2. `anchor=bottom`: the lowest opaque pixel row moves to the bottom of the cell space. Left and right stay as drawn. Use it for shrubs, rocks and small trees.
3. `anchor=trunk`: the lowest opaque row is the trunk foot. The foot moves to the bottom middle of the cell space, and the canvas is widened where the crown would be cut off, with the foot on the exact middle column. Use it for trees, so the game can place the canvas by its bottom middle.
4. `anchor=none`: cut the block as drawn.
5. The canvas is never smaller than the cell space. If the art is taller, the canvas grows upward, so the canvas bottom stays the cell bottom.
6. `scale` is applied last, nearest neighbour, inside Aseprite. The pack has 32 px tiles and Long Way has 64 px cells, so use `scale=2`.
7. The result has one layer and one frame, with the sheet's own colours. To align it with a game palette, see `adr-a8` and `remap_colors`.

## Usage

1. Run the command above, or open an existing catalogue in Aseprite.
2. Zoom in and find the sprite. Note the top left `c` and `r` and the size in tiles.
3. Ask Claude for the sprite with those values and a name.
4. Claude cuts the block from the original sheet (`cut_tiles.lua` for a block of equal tiles, or a plain crop), makes the background transparent only if it is not already, saves the sprite under the given name in the game's `assets/art/`, and records it in `games/<game>/docs/art-pipeline.md`.

## Limits

- Column and row numbers are drawn with a built-in 3x5 digit font. They are digits only, with no tile names.
- The catalogue lists positions only. It does not know which tiles form one object. A person decides that.
- A sheet with tiles that are not on a regular grid (for example a sprite sheet with different frame sizes) is better cut with `sheet_analysis` bounds than with this grid.
- The catalogue file is a derived file. It can be rebuilt at any time.

## Tests

None. Build a catalogue, export it with `aseprite -b <file> --save-as <png>`, and look at the rulers.
