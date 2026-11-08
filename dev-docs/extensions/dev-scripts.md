# Dev scripts for screenshots and art checks

All scripts take the game, file and sizes as arguments. They never change the input file, except `set_durations` with `apply=1`. Screenshots and grids go in `target/screenshots/`. Delete them after the check (`CLAUDE.md`, "Disk use").

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
| `set_durations` | `.aseprite` | `tags=a,b` or `all`, `ms=`, `apply=1`, `out=` | Sets the frame duration of those tags. A dry run (lists the frames it would change) unless `apply=1`, which saves to `out=` or over the input. Close the file in Aseprite first |
| `crop` | `.png` | `out=`, `scale=`, `cx cy w h view_height=` or `x y w h` | Cropped and scaled PNG |
| `catalogue` | sheet `.png` | `tile=` (or `tilew tileh`), `out=`, `backdrop=` | `.aseprite` copy of the sheet at 1:1 with column and row rulers and a grid. See `tile-sheet-catalogue.md` |
| `extract_block` | original sheet `.png` | `col row cols rows tile=`, `scale=`, `anchor=bottom\|trunk\|none`, `out=` | One-frame `.aseprite` of that block, anchored in its cell space and scaled. See `tile-sheet-catalogue.md` |
| `remap_colors` | `.aseprite` (RGB) | `map=from:to,...` (RRGGBB hex), `apply=1`, `out=` | Replaces exact colours in every cel. A dry run (lists pixel counts per pair and untouched colours) unless `apply=1`. Use before `extend_palette` (`adr-a8`) |
| `set_palette` | `.aseprite` (RGB) | `palette=<file>`, `apply=1` | Gives the file the palette of another file without changing a pixel. Refuses (and saves nothing) when a visible colour is not in that palette. A dry run unless `apply=1` |
| `new_sprite` | a palette `.aseprite` | `out=`, `width=`, `height=`, `fill=<palette index>`, `layer=` | Makes a new RGB file with that palette, one layer, optionally filled with one colour. Does not overwrite |
| `remove_colors` | `.aseprite` (RGB) | `colors=RRGGBB,...`, `apply=1`, `out=` | Makes every pixel of those exact colours transparent in every cel. A dry run (lists pixel counts) unless `apply=1` |

Why Lua and not `aseprite --crop`: the CLI `--crop` gave the wrong region and `--frame-range` is ignored together with `--sheet`.
