# Jet documentation

Documentation for this game only. General rules that other games may reuse are extensions, in `dev-docs/extensions/`.

| Document | Contents |
|---|---|
| [SPEC.md](SPEC.md) | The specification: rules, values, what is decided, open questions |
| [adr.md](adr.md) | Decisions that apply to Jet only (`adr-jt1` and on) |
| [art-tags.md](art-tags.md) | The palette, the `.aseprite` files, every frame and tag, and where they came from |

Status: being ported from the user's Pyxel game. The port plan is `dev-docs/PLAN-jet-port.md`.

- Run it: `./dev-scripts/rust.sh run jet [map] [--debug]`. It shows the map: the forest, the house and an empty HUD bar. There is no player yet. F1 reloads the map file, F2 toggles the info box, F3 the grid and the collision overlay. A scripted run: `./dev-scripts/rust.sh script jet "wait:1;screenshot:map;quit" --debug`.
- Map: `assets/maps/1.txt`. `F` draws the outline of a forest area and the game fills the inside (`fill=enclosed`, `dev-docs/extensions/cell-areas.md`). `H` is the house (the bottom left cell of its 4 × 4 footprint) and `@` the player start.
- Source layout (`games/jet/src/`): `main.rs` builds the app, `camera.rs` has the view and map sizes, the window size and colours, `map.rs` loads the map and names the kinds, `house.rs` spawns the house and its blocked rows, `forest.rs` makes the forest cell area and draws the trees over its cells.
- `input/` (git-ignored) holds the raw Pyxel `.aseprite` files. The game never reads it.
