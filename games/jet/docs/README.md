# Jet documentation

Documentation for this game only. General rules that other games may reuse are extensions, in `dev-docs/extensions/`.

| Document | Contents |
|---|---|
| [SPEC.md](SPEC.md) | The specification: rules, values, what is decided, open questions |
| [adr.md](adr.md) | Decisions that apply to Jet only (`adr-jt1` and on) |
| [art-tags.md](art-tags.md) | The palette, the `.aseprite` files, every frame and tag, and where they came from |

Status: being ported from the user's Pyxel game. The port plan is `dev-docs/PLAN-jet-port.md`.

- Run it: `./dev-scripts/rust.sh run jet [map] [--debug]`. It shows the map (the forest, the house, an empty HUD bar) and the player, moved with the arrow keys. Spinners come in from the field edges and hit the house; killing them is not built yet. `JET_SEED=<n>` makes a run repeatable. Script commands: `level:<n>` jumps to a difficulty level; `spinner:<column>:<row>[:<target>]` puts one Spinner on a map cell (rows from the top, as in the map file and the F3 grid), walking to target 0 to 3 (left, right, below left, below right of the house) or a random one. A Spinner that stands still for 0.2 s is logged at debug level (`enemy: ... stuck at`) with its cell, target and path. F1 reloads the map file, F2 toggles the info box, F3 the grid and the collision overlay. A scripted run: `./dev-scripts/rust.sh script jet "wait:1;screenshot:map;quit" --debug`.
- Map: `assets/maps/1.txt`. `F` draws the outline of a forest area and the game fills the inside (`fill=enclosed`, `dev-docs/extensions/cell-areas.md`). `H` is the house (the bottom left cell of its 4 × 4 footprint) and `@` the player start.
- Source layout (`games/jet/src/`): `main.rs` builds the app, `camera.rs` has the view and map sizes, the window size and colours, `map.rs` loads the map and names the kinds, `house.rs` spawns the house and its blocked rows, `forest.rs` makes the forest cell area and draws the trees over its cells, `input.rs` has the key rules (direction and fallback), `player.rs` moves the player, finds what blocked it, asks for the camera shake when it hits a wall and shows the move animation, trail and shockwave, `enemy.rs` has the enemy kinds, the `RaidHouse` strategy and the house hit, `spawning.rs` has the difficulty levels and the field spawner.
- `input/` (git-ignored) holds the raw Pyxel `.aseprite` files. The game never reads it.
