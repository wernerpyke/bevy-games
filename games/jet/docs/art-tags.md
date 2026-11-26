# Jet: art files and tags

Every file in `games/jet/assets/art/` (`adr-jt5`, `adr-jt13`). The `.aseprite` files are the source of truth. Edit them in Aseprite; the game loads them directly.

## Common rules

- RGB colour mode, on the palette of `palette.aseprite`. Check with `./dev-scripts/aseprite.sh check <file> palette=games/jet/assets/art/palette.aseprite`.
- Thin white line art on transparent. The art layers are visible. Each file has a hidden layer `backdrop` (black) at the bottom, for drawing on black in Aseprite. The game draws only visible layers (the `bevy_aseprite_ultra` default), so it never draws the backdrop. Keep it hidden.
- Art is at 2× the Pyxel size: 1 Pyxel pixel is 2 × 2 pixels, so one 8 px Pyxel tile is one 16 px cell.
- Frame durations are in the files (`adr-a6`): 62 ms (16 fps) for fast animations, 250 ms (4 fps) for the house.
- Sprites fill their cells, so most frames touch the canvas edge. That is expected.
- Player, trail and shockwave art face **up**. The game rotates them for the other directions.

## Palette: `palette.aseprite`

64 × 8, one 8 px swatch per entry, layer `Swatches`. Index 0 is transparent.

| Index | Hex | Used for |
|---|---|---|
| 0 | transparent | |
| 1 | `#000000` | Ground, backdrop |
| 2 | `#EEEEEE` | All line art |
| 3 | `#A3A3A3` | Spinner `Die` |
| 4 | `#D4186C` | Spinner `Kill`; house damage 2 |
| 5 | `#FF9798` | House damage 1 |
| 6 | `#7E2072` | House damage 3 |
| 7 | `#19959C` | Player while a heal is charged |

More colours are added with the particles. Add a colour to this file first, then use it.

## Files

| File | Canvas | Frames | Layers | Tags |
|---|---|---|---|---|
| `player.aseprite` | 16 × 16 | 9 | `backdrop` (hidden), `art` | `Idle` 1 (62 ms), `Move` 2–5, `Trail` 6–9 |
| `spinner.aseprite` | 16 × 16 | 8 | `backdrop` (hidden), `art` | `Move` 1–4, `Die` 5–6 (grey), `Kill` 7–8 (red X) |
| `house.aseprite` | 64 × 64 | 4 | `backdrop` (hidden), `art`, `smoke` | `Smoke` 1–4 (250 ms). The chimney smoke and a small shift of the window change between frames. |
| `trees.aseprite` | 32 × 32 | 8 | `backdrop` (hidden), `art` | None. Each frame is one tree variant, shown as a still frame. The Pyxel game picks frame 5 twice as often as the others; the game code does that. |
| `shockwave.aseprite` | 48 × 16 | 4 | `backdrop` (hidden), `art` | `Wave` 1–4 |

Tag use in the game (from `SPEC.md`):

- `player`: `Idle` while still (not rotated). `Move` plays once on a start or a turn, rotated, and holds its last frame. `Trail` loops on a separate sprite one cell behind the player.
- `spinner`: `Move` loops while walking (not rotated). `Die` plays once when the player kills it. `Kill` plays once when it hits the house.
- `house`: `Smoke` loops.
- `shockwave`: `Wave` loops one cell ahead of the moving player, rotated.

## Where the files came from

Built on 2026-11-26 from the Pyxel game's files (`~/Dark/pyxel/games/jet/assets/`), copied into `games/jet/input/` (git-ignored), with a one-off Lua script (not kept):

- `sprites.aseprite` (8 × 16, 9 frames) held two sprites per frame: the top half of frames 1–9 was player idle, player move (2–5) and Spinner move (6–9). The bottom half was the trail (under frames 2–5), Spinner die (6–7) and Spinner kill (8–9). The bottom of frame 1 was empty. It was split into `player.aseprite` and `spinner.aseprite`, one 8 × 8 sprite per frame. The brief puts the trail at sheet columns 1–4; it is really under columns 2–5, and the split follows the file. The art pixel count was checked: 262 before, 262 after.
- `house.aseprite`, `forest.aseprite` (now `trees.aseprite`) and `shockwave.aseprite`: art layers kept (`layer` renamed to `art`).
- In every file: the visible `Background` layer (the Pyxel transparent beige, navy in `sprites`) and the `bg` layer (beige or black) were removed, and the hidden `backdrop` was added. The old `Loop` tags were removed. Durations were set from 100 ms to the Pyxel speeds. Then the canvas was resized 2× (nearest neighbour), and the palette was set.
- Every colour was already one of the palette colours, so no pixel colour changed.
