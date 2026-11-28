# Jet: art files and tags

Every file in `games/jet/assets/art/` (`adr-jt5`, `adr-jt13`). The `.aseprite` files are the source of truth. Edit them in Aseprite; the game loads them directly.

## Common rules

- RGB colour mode, on the palette of `palette.aseprite`. Check with `./dev-scripts/aseprite.sh check <file> palette=games/jet/assets/art/palette.aseprite`.
- Black line art on transparent, on a snow ground (index 14, `#F4F8FB`), since 2026-11-28 (was white line art on black, `adr-jt5`). The trees are coloured. The art layers are visible. Each file has a hidden layer `backdrop` (the snow colour) at the bottom, for drawing on black in Aseprite. The game draws only visible layers (the `bevy_aseprite_ultra` default), so it never draws the backdrop. Keep it hidden.
- A cell is 32 px (`adr-jt2`, 2026-11-28). The sprites from the Pyxel game are at 4× the Pyxel size: 1 Pyxel pixel is 4 × 4 pixels, so one 8 px Pyxel tile is one 32 px cell. The trees are drawn new for 32 px cells.
- Frame durations are in the files (`adr-a6`): 62 ms (16 fps) for fast animations, 250 ms (4 fps) for the house.
- Sprites fill their cells, so most frames touch the canvas edge. That is expected.
- Player, trail and shockwave art face **up**. The game rotates them for the other directions.
- **The sprites will get a more 3D look over time.** The user will update the existing sprites by hand, a little at a time, so for a while some sprites will be flat line art and some shaded. Do not redraw, recolour or regenerate a sprite: the user owns these changes. When a file changes, update its row here (layers, tags, colours) and check that its tags still match the code.

## Palette: `palette.aseprite`

168 × 8, one 8 px swatch per entry, layer `Swatches`. Index 0 is transparent.

| Index | Hex | Used for |
|---|---|---|
| 0 | transparent | |
| 1 | `#000000` | All line art (since 2026-11-28) |
| 2 | `#EEEEEE` | Was the line art until 2026-11-28; not used now |
| 3 | `#A3A3A3` | Spinner `Die` |
| 4 | `#D4186C` | Spinner `Kill`; house damage 2 |
| 5 | `#FF9798` | House damage 1 |
| 6 | `#7E2072` | House damage 3 |
| 7 | `#19959C` | Player while a heal is charged |
| 8 | `#FFEDE4` | Trees: snow highlight |
| 9 | `#92BED2` | Trees: snow shade |
| 10 | `#4B3418` | Trees: bark |
| 11 | `#2F1A0F` | Trees: dark bark |
| 12 | `#645122` | Trees: bark light |
| 13 | `#9D8231` | Trees: bark highlight |
| 14 | `#F4F8FB` | Snow: the ground (`CLEAR_COLOR`) and the backdrops |
| 15 | `#1E110A` | Dark wood: deepest |
| 16 | `#5A3A22` | Dark wood: warm mid |
| 17 | `#1C3326` | Dark leaves: deep |
| 18 | `#2F5A3C` | Dark leaves: mid |
| 19 | `#C4DDE9` | Ice: light |
| 20 | `#5F8FA8` | Ice: deep |

More colours are added with the particles. Add a colour to this file first, then use it.

## Files

| File | Canvas | Frames | Layers | Tags |
|---|---|---|---|---|
| `player.aseprite` | 32 × 32 | 9 | `backdrop` (hidden), `art` | `Idle` 1 (62 ms), `Move` 2–5, `Trail` 6–9 |
| `spinner.aseprite` | 32 × 32 | 8 | `backdrop` (hidden), `art` | `Move` 1–4, `Die` 5–6 (grey), `Kill` 7–8 (red X) |
| `house.aseprite` | 128 × 128 | 4 | `backdrop` (hidden), `art` | `Idle` 1–4 (250 ms; named `Smoke`, with a separate `smoke` layer, until 2026-11-28). The chimney smoke and a small shift of the window change between frames. |
| `trees.aseprite` | 64 × 64 | 4 | `backdrop` (hidden), `art` | None. Each frame is one tree variant (2 × 2 cells), shown as a still frame, each picked as often. Drawn by the user (2026-11-26, as `trees2.aseprite`); it replaced the Pyxel trees (32 × 32, 8 variants) on 2026-11-28. |
| `shockwave.aseprite` | 96 × 32 | 4 | `backdrop` (hidden), `art` | `Wave` 1–4 |

Tag use in the game (from `SPEC.md`):

- `player`: `Idle` while still (not rotated). `Move` plays once on a start or a turn, rotated, and holds its last frame. `Trail` loops on a separate sprite one cell behind the player.
- `spinner`: `Move` loops while walking (not rotated). `Die` plays once when the player kills it. `Kill` plays once when it hits the house.
- `house`: `Idle` loops.
- `shockwave`: `Wave` loops one cell ahead of the moving player, rotated.

## Where the files came from

Built on 2026-11-26 from the Pyxel game's files (`~/Dark/pyxel/games/jet/assets/`), copied into `games/jet/input/` (git-ignored), with a one-off Lua script (not kept):

- `sprites.aseprite` (8 × 16, 9 frames) held two sprites per frame: the top half of frames 1–9 was player idle, player move (2–5) and Spinner move (6–9). The bottom half was the trail (under frames 2–5), Spinner die (6–7) and Spinner kill (8–9). The bottom of frame 1 was empty. It was split into `player.aseprite` and `spinner.aseprite`, one 8 × 8 sprite per frame. The brief puts the trail at sheet columns 1–4; it is really under columns 2–5, and the split follows the file. The art pixel count was checked: 262 before, 262 after.
- `house.aseprite`, `forest.aseprite` (now `trees.aseprite`) and `shockwave.aseprite`: art layers kept (`layer` renamed to `art`).
- In every file: the visible `Background` layer (the Pyxel transparent beige, navy in `sprites`) and the `bg` layer (beige or black) were removed, and the hidden `backdrop` was added. The old `Loop` tags were removed. Durations were set from 100 ms to the Pyxel speeds. Then the canvas was resized 2× (nearest neighbour), and the palette was set.
- Every colour was already one of the palette colours, so no pixel colour changed.
- 2026-11-28: `player`, `spinner`, `house` and `shockwave` resized 2× again (nearest neighbour, `dev-scripts/aseprite/resize.lua`) for 32 px cells. Tags and durations checked unchanged.
- 2026-11-28: the Pyxel `trees.aseprite` was deleted and the user's `trees2.aseprite` renamed to `trees.aseprite`. Its 6 colours were added to the palette as indexes 8 to 13 (`extend_palette.lua`; the swatches drawn with a one-off script).
- 2026-11-28: black line art on snow. In `player`, `spinner`, `house` and `shockwave` the line art `#EEEEEE` became `#000000` (`remap_colors.lua`). The backdrops became the snow colour `#F4F8FB`, added to the palette as index 14 (tried `#E6EDF2` and `#E9EDF0` first): the house backdrop from pink, the trees backdrop from black.