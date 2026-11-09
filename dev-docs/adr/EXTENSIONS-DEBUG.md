# Extensions: debug and dev tools decisions

Debug overlays, scripted runs, logging and the debug keys. Index of all extension decisions: `EXTENSIONS.md`.

## adr-x3. Collision debug overlay (2026-10-31, changed 2026-11-01)

**Decision.** A debug overlay shows the collision. First version: F1, gizmos, red trunk boxes and green feet circles. Now (`collision_debug.rs`): it follows the F3 grid switch and is drawn as sprites on the feature debug layer (`adr-x9`). Yellow lines are every outline edge (with a tick on the blocked side), cyan boxes are interior rocks, green squares are the feet of every depth-sorted entity. The red trunk boxes are gone because the yellow outline already covers trunks. F1 does nothing in the kit. R-Town uses F1 to reload its map (`adr-x19`).

**Rationale.** Collision rules are hard to check by eye, and Claude cannot watch the window.

**How to apply.** Keep the overlay while collision work continues. Remove it or put it behind a debug feature before any release build.

## adr-x5. Debug strategy: logging and scripted input (2026-10-31)

**Decision.** Behaviour is checked in two ways besides unit tests:
1. **Logging.** Game code logs state changes, not every frame, with `debug!` under the crate's module path (for example `long_way::collision`). `info!` is for script progress. Enable with `RUST_LOG=long_way=debug`.
2. **Scripted input.** If the `LONG_WAY_SCRIPT` environment variable is set, `script.rs` (now in `crates/game-kit`, adr-x15) presses keys in the running game and logs the feet position of the controlled entity every 0.25 seconds. Commands: `teleport:<x>:<y>`, `hold:<keys>:<seconds>`, `wait:<seconds>`, `screenshot:<name>`, `quit`. `screenshot` saves the window to `target/screenshots/<name>.png` (Bevy `Screenshot` with `save_to_disk`, native only) so Claude can open the PNG and check what is drawn (added 2026-11-01). Use `wait` before it after a `teleport`. It does nothing when the variable is not set. Run it with `./dev-scripts/rust.sh script long-way "<commands>"`.

**Rationale.** Unit tests passed for `allow_sliding` but the feature did not work in the real game. Claude cannot play the game, so it needs a way to reproduce a problem in the real app, with real frame times. The scripted run reproduced it, and the log lines ("blocked" repeating) showed the cause: the push direction was compared as floats, and rounding noise between frames kept resetting the push timer. A unit test with varying frame times now covers it.

**How to apply.** When a behaviour does not work in the real game, first reproduce it with a script, then write a failing test that captures the cause, then fix it, then rerun the script. Do not report a fix as working from unit tests alone. Do not compare floats for equality to detect a change of state: compare signs or discrete values. Keep logs to transitions so a run stays readable. New game logic that needs this kind of check should log its own transitions in the same way.


## adr-x6. Reusable visual debug tools in `debug.rs` (2026-11-01)

**Decision.** `crates/game-kit/src/debug.rs` (moved from Long Way on 2026-11-02, `adr-x12`) holds visual debug tools, one tool per system, added over time. `DebugPlugin { cell_size, grid_color }` is configured in `main.rs`. The first two tools:
1. **Grid (F3).** Lines at every `cell_size` (64) in `grid_color` (red), over the visible area, drawn as thin sprites at `GRID_Z` (-10): below the player and the trees (about -1 to 1), above a future background, which must use a lower z. Gizmos are not used because they always draw on top. Column numbers run along the top edge and row numbers along the left edge. `grid_origin` sets where (0, 0) is: `Center`, `BottomLeft` (used by Long Way) or `World(Vec2)`. `BottomLeft` is the corner of the visible area, and the view width follows the window aspect ratio, so that corner moves sideways in world units when the window is resized. Use `World(Vec2)` for a fixed origin.
2. **Info box (F2).** A text box at the top left. Any system shows a value with `ResMut<DebugInfo>` and `info.set("label", value)`. Rows keep the order of first use. The first rows are the player's feet x/y and grid cell (in grid coordinates, relative to the origin) and its raw world position.

Both start visible. The collision overlay is in `collision_debug.rs` and follows F3 (`adr-x3`).

**Rationale.** The developer wants one place to add debug visuals as needs come up. The `DebugInfo` resource lets other plugins report values without calling into `debug.rs` directly. The player's feet are shown, not the sprite centre, because collision uses the feet.

**How to apply.** Add a new tool as a new system in `DebugPlugin`, with a key toggle in `toggle_tools` and a field in `DebugSettings`. Report values from any system through `DebugInfo`. The code stays in the game crate until a second game needs it (`adr-w6`). Remove or gate the tools behind a debug feature before a release build.

## adr-x15. Scripted run tool is in the kit and extensible (2026-11-02)

**Decision.** The kit owns the script parser and runner (teleport, hold, wait, screenshot, quit). The game marks its target entity with a `ScriptTarget` component and can register its own commands.

**Rationale.** Both games get scripted runs and screenshots without copying (adr-x5). R-Town will need game commands such as attacks once combat exists.

**How to apply.** Game commands are added through a registration API in the kit. Design that API with the first game command, not before.

## adr-x19. Debug keys F2 info, F3 grid, F1 map reload (2026-11-03)

**Decision.** The kit debug keys are F2 for the info box and F3 for the grid and the collision overlay (they were the other way round). (Revised by `adr-x28`: the kit now reads F1 and sends `ReloadMap`.) The collision overlay redraws when `CollisionWorld` changes, and `CollisionWorld::clear` lets a game replace its blocked areas. The script runner accepts `f1` to `f3` in `hold`, so a reload can be tested in a scripted run.

**Rationale.** The developer wanted reload first, then the info box, then the grid, in the order they use them while editing a map. A reload must redraw the overlay, or the yellow outline shows the old map.

**How to apply.** A game that reloads a map despawns what the map spawned, replaces the resources, spawns again, then calls `clear` and `add_blocked` with the new blockers.

## adr-x28. F1 sends `ReloadMap` from the kit, and `--debug` turns the debug tools on (2026-11-04)

**Decision.** `game_kit::reload::ReloadPlugin` reads F1 and sends a `ReloadMap` message. The game reads it (R-Town: `reload_map.run_if(on_message::<ReloadMap>)`) and decides what a reload resets. The kit does not reload anything itself. The debug tools (F2 info box, F3 grid, collision and movement overlays) are off unless the run starts with `--debug`: `rust.sh run|script <game> --debug` sets `GAME_KIT_DEBUG=1`, and `DebugPlugin::start_visible` reads it. Without the flag the overlays start hidden, and F2 and F3 still toggle them.

**Rationale.** The key belongs with the other debug keys, and a second game can reuse it without copying the binding. What a reload resets is game state (R-Town removes enemies and projectiles), so a message keeps the kit free of game types. Screens and screenshots are clean by default, and Claude asks for the overlays when it needs them. The keys keep working so the developer can still show them in a normal run. An environment variable fits the existing `<GAME>_SCRIPT` and `<GAME>_MAP` pattern and needs no argument parsing in the games.

**How to apply.** A game adds `ReloadPlugin` and a system that runs on `ReloadMap`. Long Way reloads on it too (`map.rs`: rocks, trees and the player are tagged `MapEntity`, removed and spawned again). Claude adds `--debug` when testing or taking screenshots (`CLAUDE.md`). A web build has no environment, so the tools start hidden there and F2 and F3 show them. This replaces the part of `adr-x19` that left F1 to the game.

