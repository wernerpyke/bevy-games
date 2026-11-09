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

# starts with the F2 info box and the F3 grid visible
./dev-scripts/rust.sh run r-town --debug

# skips the title screen (a game with a title screen only; scripted runs always skip it)
./dev-scripts/rust.sh run long-way --skip-title
```

The first build takes several minutes.

## Controls

- Long Way: click with the left mouse button to walk to that place. The player goes around trees and rocks, and a click on a blocked place goes to the nearest place it can reach. A new click changes the goal. The arrow keys do nothing there.
- R-Town: arrow keys walk. X fires a projectile (R-Town, it flies the way the player faces). F1 reloads the map and resets the game (R-Town), F2 toggles the info box, F3 toggles the grid and the movement debug (they start hidden unless the game runs with `--debug`). Enemies (R-Town) kill the player on a touch: press F1 to start again.

## Troubleshooting

- Asset not found when running the binary directly: run it through `./dev-scripts/rust.sh run <game>`, or set `BEVY_ASSET_ROOT` to the game folder (for example `games/long-way`).
- The game stops at start with a message like `assets/maps/test.txt:15: row has 157 characters between the bars but 159 are needed for 80 cells`: the map file has a mistake on that line. The message names the file and line. Common causes are a missing or extra space in a row, a wrong row label, a character that is not in the legend, and a row count that does not match `size`. The format is in `dev-docs/extensions/map-files.md`.
- Map rows look broken in the editor: use a fixed-width font and turn off word wrap. A blank cell is a space, so do not let the editor remove spaces before the closing `|`.
- I can walk through a tree, or cannot walk behind one: an item that touches another item (also at a corner) is part of a solid group. An item with no neighbour can be walked behind. Move or remove the neighbour.
- The map does not change after an edit (R-Town): press F1 in the game. F1 also puts the player back at the start and removes the enemies and projectiles. If the file has an error, the game logs it and keeps the old map. The web build cannot reload.
- Cell numbers in the map file and on the F3 grid differ: the file counts rows from its `origin` corner. R-Town uses `top-left`, and its F3 grid counts from the top to match. Long Way uses `bottom-left`.
- `target/` takes a lot of disk space: run `./dev-scripts/rust.sh size` to see the sizes and `./dev-scripts/rust.sh prune` to delete the incremental cache and stale binaries. A clean build is about 2.2 GB. If `target/` is over 8 GB after a prune, run `cargo clean` and build again (about 10 minutes). Dev builds keep no debug info for dependencies, so a debugger cannot step into Bevy code.
