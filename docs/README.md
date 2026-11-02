# Games

A Cargo workspace of pixel art games built with Bevy. Each game lives in `games/<name>/`.

| Game | Folder |
|---|---|
| Long Way | `games/long-way/` |
| R-Town | `games/r-town/` |

Single entry point for players and users: quick start, controls, common commands and troubleshooting.

## Quick start

```
./dev-scripts/rust.sh run long-way
./dev-scripts/rust.sh run r-town
```

The first build takes several minutes.

## Controls

- Arrow keys: walk.

## Troubleshooting

- Asset not found when running the binary directly: run it through `./dev-scripts/rust.sh run <game>`, or set `BEVY_ASSET_ROOT` to the game folder (for example `games/long-way`).
- The game stops at start with a message like `assets/maps/test.txt:15: row has 157 characters between the bars but 159 are needed for 80 cells`: the map file has a mistake on that line. The message names the file and line. Common causes are a missing or extra space in a row, a wrong row label, a character that is not in the legend, and a row count that does not match `size`. The format is in `dev-docs/extensions/map-files.md`.
- Map rows look broken in the editor: use a fixed-width font and turn off word wrap. A blank cell is a space, so do not let the editor remove spaces before the closing `|`.
- Cell numbers in the map file and on the F2 grid differ: the file counts rows from its `origin` corner. R-Town uses `top-left`, and its F2 grid counts from the top to match. Long Way uses `bottom-left`.
