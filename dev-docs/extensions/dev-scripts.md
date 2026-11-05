# Dev scripts for screenshots and art checks

All scripts take the game, file and sizes as arguments. They never change the input file. Screenshots and grids go in `target/screenshots/`. Delete them after the check (`CLAUDE.md`, "Disk use").

## Zoomed screenshot

`./dev-scripts/shot.sh <game> <name> "<commands>" [cx= cy= w= h= scale=]`

- Runs `rust.sh script` with `--debug`, adds `screenshot:<name>;quit`, then crops.
- `cx`, `cy`: world centre of the crop (y up). `w`, `h`: world size. `scale`: whole-number zoom.
- The conversion is `screen = window centre + world * (window height / VIEW_HEIGHT)`, y flipped. `VIEW_HEIGHT` comes from the game's `camera.rs`.
- Output: `<name>.png` (whole window) and `<name>_crop.png`.

## Aseprite checks

`./dev-scripts/aseprite.sh <script> <file> [key=value ...]` runs `dev-scripts/aseprite/<script>.lua` with the Aseprite CLI (`ASEPRITE` sets another binary).

| Script | Input | Parameters | Result |
|---|---|---|---|
| `check` | `.aseprite` | `palette=<file>`, `edge=<px>` | Canvas, colour mode, layers, tags (frames, durations), colours outside the palette, bounds and feet row per frame, frames cut off at the edge |
| `frame_grid` | `.aseprite` | `out=`, `tag=` or `frames=a-b`, `cols=`, `scale=`, `gap=` | One PNG grid of the visible layers |
| `sheet_analysis` | sheet `.png` | `frame=WxH` | Bounds and colour count per frame, all colours of the sheet |
| `crop` | `.png` | `out=`, `scale=`, `cx cy w h view_height=` or `x y w h` | Cropped and scaled PNG |

Why Lua and not `aseprite --crop`: the CLI `--crop` gave the wrong region and `--frame-range` is ignored together with `--sheet`.
