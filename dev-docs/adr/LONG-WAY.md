# Long Way: extension decisions

Decisions about reusable code that Long Way implements first. Game only decisions (`adr-lw1`, `adr-lw7`, `adr-lw8`, `adr-lw12`, `adr-lw13`) are in `games/long-way/docs/adr.md`. IDs are shared by both files.

## adr-lw2. Draw order sorted by feet (2026-10-31)

**Decision.** Entities with a `DepthSort` component get their z set from the y position of their feet: the lower the feet on screen, the closer to the camera.

**Rationale.** Guy must draw behind a tree when his feet are above its base and in front when below. A fixed z cannot do both. The same sort line also decides collision, so what the player sees matches where he can walk.

**How to apply.** Give any sprite that should sort a `DepthSort { feet_offset }`, the distance from its centre down to its feet. `depth.rs` sets `z = -feet_y * 0.001`. Sprites with the same feet y are ordered by x (`z += x * 1e-8`), because equal z flickers (found with the rock rows, 2026-11-01). The sort runs in `Update` with no ordering against movement, so z can lag one frame.

## adr-lw3. Collision debug overlay (2026-10-31, changed 2026-11-01)

**Decision.** A debug overlay shows the collision. First version: F1, gizmos, red trunk boxes and green feet circles. Now (`collision_debug.rs`): it follows the F2 grid switch and is drawn as sprites on the feature debug layer (`adr-lw9`). Yellow lines are every outline edge (with a tick on the blocked side), cyan boxes are interior rocks, green squares are the feet of every depth-sorted entity. The red trunk boxes are gone because the yellow outline already covers trunks. F1 does nothing now.

**Rationale.** Collision rules are hard to check by eye, and Claude cannot watch the window.

**How to apply.** Keep the overlay while collision work continues. Remove it or put it behind a debug feature before any release build.

## adr-lw4. Forgiving collision: `allow_sliding` (2026-10-31)

**Decision.** `CollisionPlugin` takes an `allow_sliding` flag. Long Way sets it to `true`. When true:
1. Guy slides along a trunk when walking diagonally into it.
2. Walking straight into a face stops Guy, which shows the collision.
3. If he keeps pushing for 0.2 seconds (first 0.5, shortened on 2026-10-31 after the developer tried it) and his feet are within 35% of the face length from an edge (capped at 64 pixels, see `adr-lw10`, which also covers faces of merged outlines and notches), he is slid sideways at 120 units per second to that edge, then walks on past the trunk. This works from below (slide left or right), from above, and into the trunk sides (slide up or down).
Pushing into the middle of a face never slides him, so he must walk around.

When false, any blocked move leaves Guy where he was. There is no sliding of any kind.

**Rationale.** Hitting a trunk by a few pixels should not make the player back off and line up again. The pause makes the collision visible before the game helps. Pushing into the middle is a clear signal that the player means to reach the trunk, so no help is given there.

**How to apply.** `SLIDE_DELAY`, `SLIDE_SPEED` and `SLIDE_RANGE_FRACTION` in `collision.rs` set the numbers. Entities that move with `move_feet` need a `SlideAssist` component, and call `reset()` on it when they are not trying to move. The setting is a resource (`CollisionSettings`) set by the plugin. Assisted sliding only starts when the input is straight along one axis. The collision rule from `adr-lw1` is unchanged. The push direction is stored as signs (for example (0, 1)), not as a normalised float, because comparing floats made the timer reset every few frames (`adr-lw5`).

## adr-lw5. Debug strategy: logging and scripted input (2026-10-31)

**Decision.** Behaviour is checked in two ways besides unit tests:
1. **Logging.** Game code logs state changes, not every frame, with `debug!` under the crate's module path (for example `long_way::collision`). `info!` is for script progress. Enable with `RUST_LOG=long_way=debug`.
2. **Scripted input.** If the `LONG_WAY_SCRIPT` environment variable is set, `script.rs` presses keys in the running game and logs Guy's feet position every 0.25 seconds. Commands: `teleport:<x>:<y>`, `hold:<keys>:<seconds>`, `wait:<seconds>`, `screenshot:<name>`, `quit`. `screenshot` saves the window to `target/screenshots/<name>.png` (Bevy `Screenshot` with `save_to_disk`, native only) so Claude can open the PNG and check what is drawn (added 2026-11-01). Use `wait` before it after a `teleport`. It does nothing when the variable is not set. Run it with `./dev-scripts/rust.sh script long-way "<commands>"`.

**Rationale.** Unit tests passed for `allow_sliding` but the feature did not work in the real game. Claude cannot play the game, so it needs a way to reproduce a problem in the real app, with real frame times. The scripted run reproduced it, and the log lines ("blocked" repeating) showed the cause: the push direction was compared as floats, and rounding noise between frames kept resetting the push timer. A unit test with varying frame times now covers it.

**How to apply.** When a behaviour does not work in the real game, first reproduce it with a script, then write a failing test that captures the cause, then fix it, then rerun the script. Do not report a fix as working from unit tests alone. Do not compare floats for equality to detect a change of state: compare signs or discrete values. Keep logs to transitions so a run stays readable. New game logic that needs this kind of check should log its own transitions in the same way.


## adr-lw6. Reusable visual debug tools in `debug.rs` (2026-11-01)

**Decision.** `games/long-way/src/debug.rs` holds visual debug tools, one tool per system, added over time. `DebugPlugin { cell_size, grid_color }` is configured in `main.rs`. The first two tools:
1. **Grid (F2).** Lines at every `cell_size` (64) in `grid_color` (red), over the visible area, drawn as thin sprites at `GRID_Z` (-10): below Guy and the trees (about -1 to 1), above a future background, which must use a lower z. Gizmos are not used because they always draw on top. Column numbers run along the top edge and row numbers along the left edge. `grid_origin` sets where (0, 0) is: `Center`, `BottomLeft` (used by Long Way) or `World(Vec2)`. `BottomLeft` is the corner of the visible area, and the view width follows the window aspect ratio, so that corner moves sideways in world units when the window is resized. Use `World(Vec2)` for a fixed origin.
2. **Info box (F3).** A text box at the top left. Any system shows a value with `ResMut<DebugInfo>` and `info.set("label", value)`. Rows keep the order of first use. The first rows are Guy's feet x/y and grid cell (in grid coordinates, relative to the origin) and his raw world position.

Both start visible. The collision overlay is in `collision_debug.rs` and follows F2 (`adr-lw3`).

**Rationale.** The developer wants one place to add debug visuals as needs come up. The `DebugInfo` resource lets other plugins report values without calling into `debug.rs` directly. Guy's feet are shown, not the sprite centre, because collision uses the feet.

**How to apply.** Add a new tool as a new system in `DebugPlugin`, with a key toggle in `toggle_tools` and a field in `DebugSettings`. Report values from any system through `DebugInfo`. The code stays in the game crate until a second game needs it (`adr-w6`). Remove or gate the tools behind a debug feature before a release build.

## adr-lw9. Draw order layers (2026-11-01)

**Decision.** Everything on screen belongs to a named layer, from back to front, defined in one file per game (`layers.rs`). Sprite layers are bands of z. HUD and UI layers use `GlobalZIndex`. Features and characters share one world layer, sorted together by their feet (`depth.rs`). Debug drawings have their own layers above the world. The layers Long Way uses are in `adr-lw12` (`games/long-way/docs/adr.md`).

**Rationale.** Bevy has no layer system for 2D draw order: sprites are sorted by z, equal z has no fixed order, `RenderLayers` only picks which camera sees an entity, and 2D gizmos are always drawn last, on top of everything. The developer asked for formal layers on 2026-11-01. Features and characters share one layer because a character must walk behind a tree or rock when its feet are above the feature's sort line (`adr-lw1`, `adr-lw2`). Separate layers would stop that. The debug layer is above the world, so debug lines also draw over the player.

**How to apply.** Never write a bare z number outside `layers.rs`. `bevy_ui` draws after all sprites and gizmos, so the HUD and UI are always above the feature debug layer. A background must use a z below `GRID_DEBUG_Z`. `depth.rs` has a debug assertion that feet stay inside the world band (y within about 100000). Debug drawings that must follow the layers are sprites, not gizmos. The collision overlay in `collision_debug.rs` follows this rule.

## adr-lw10. Sliding along merged outlines, into notches (2026-11-01)

**Decision.** Blocked rectangles that touch or overlap are merged into one outline per cluster (`outline.rs`). Guy collides with the outline edges only, and `allow_sliding` (`adr-lw4`) works on those edges. The slide rules for an outline are:
1. The range is the smaller of 35% of the face and 64 pixels (`SLIDE_RANGE_CAP`), measured from the nearer end.
2. An end at an inward (concave) corner never slides Guy. Past it there is only solid ground.
3. At a convex end, Guy always slides. If the way on is open for 192 pixels (`SLIDE_CLEARANCE`) he stops exactly on the end. If a face blocks it (a notch or an overhang), he stops 0.5 pixels past the end (`SLIDE_PAST_CORNER`) and walks on until that face stops him. Example: pushing up into the bottom face of the test cluster from world x 463, Guy slides to x 440.3 and walks up to y 64.
4. All blocked areas (trunks and rocks) are merged by one `outline::build`, so a trunk touching a rock is one cluster. Two rectangles that touch only at a corner leave no gap: that vertex counts as an inward corner. A slide target is passed through `resolve`, so it never ends inside another shape.

The blocked area and sort line of a rock are game rules, in `adr-lw13` (`games/long-way/docs/adr.md`).

**Rationale.** The first version refused to slide when the way past the end was blocked within 192 pixels, to avoid pushing Guy into pockets. The developer tried it at a notch under an overhang and wanted him to slide to the corner and walk on, stopping at the back of the notch (2026-11-01). Sliding into an inward corner is still refused, because it only puts Guy inside the next rock.

**How to apply.** The rules are in `slide_to_nearest_edge` in `collision.rs`, with tests for pockets, notches and long faces. The numbers were accepted by the developer after play testing (2026-11-02). The full rules are in `dev-docs/extensions/collision.md`, the Long Way values in `games/long-way/docs/collision.md`.

## adr-lw11. Map file layout: Features and Map sections (2026-11-01)

**Decision.** A map file is markdown with frontmatter and two sections the game reads (`dev-docs/extensions/map-files.md`):
1. `# Features`: each `## Name` heading defines a feature with exactly one `cells` block. The bottom left character is the anchor.
2. `# Map`: a bulleted list that places features, `- 'Name' at (x,y), (x,y) and (x,y)`. Positions are the anchor cell, in F2 grid numbers.
Everything else in the file is notes for people. Trees are map kinds (`green_tree`, `red_tree`) placed the same way as rocks. Every format error stops loading and names the file and line.

**Rationale.** The first format put `at=x,y` on each `cells` block, with a separate `name=` and `place` block for reuse. The developer wanted features described once, with comments, and placement in one readable list. A feature used several times is one bullet. It also lets the same file place trees, which need no change to the format, only a new kind in code.

**How to apply.** The parser is `parse` in `map.rs` (plain Rust, unit tested). Add a new object kind in `Kind` and in the module that spawns it (`rocks.rs`, `trees.rs`), and name it in the legend. Do not add per-block options to `cells`. Roadmap item 5 considers other formats (YAML, TOML) if maps outgrow markdown. Roadmap item 4 covers a sturdier loader.
