# Long Way: decisions

## adr-lw1. Custom tree collision with a blocked band, no physics crate (2026-10-31)

**Decision.** Trees block Guy with a hand-written rule, not a physics engine. Guy's feet are blocked from entering a rectangle around each trunk. The rectangle is:
- **Sides:** the trunk's left and right edges, widened by Guy's half width (20), so his body cannot overlap the trunk from left or right.
- **Bottom:** the tree's base minus half of Guy's height (30). Walking up from below, Guy stops with the top half of his body over the trunk base (50% overlap).
- **Top:** the tree's base plus 10% of the tree height. Walking down from above, Guy stops before the bottom 10% of the tree. Above this line he can walk behind the tree, overlapping the trunk and canopy freely.

**Rationale.** These are the developer's rules for a top-down view, and they are not a normal box collision. A rectangle test on feet positions expresses all three rules with four numbers. The game is a real-time top-down RPG (`adr-lw7`) and these rules are not physics, so a physics engine is not wanted for now (roadmap item 4 evaluates `avian2d`).

**How to apply.** The rule lives in `games/long-way/src/collision.rs` as plain functions with unit tests, with no Bevy types except `Vec2` and `Rect`. Constants (`GUY_HALF_WIDTH`, `GUY_HEIGHT`, `OVERLAP_FROM_BELOW`, `BLOCKED_BOTTOM`) set the numbers. Trunk edges come from the art (`TRUNK_LEFT_PX` and `TRUNK_RIGHT_PX` in `trees.rs`). Each axis is resolved separately so Guy slides along a trunk, and a move is stopped when it crosses an edge, so a large step cannot jump over a trunk.

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

## adr-lw7. Long Way is a real-time top-down RPG (2026-11-01)

**Decision.** Long Way is a real-time, top-down RPG. It is not turn-based and not locked to a grid. Movement, sliding and animation use frame time. The 64 pixel cell is a tool for placing things (the F2 grid and map files), not a movement rule.

**Rationale.** The developer chose this direction on 2026-11-01. It replaces the earlier workspace default of turn-based play for this game. Walking, collision and the slide delay were already real-time, so no existing code changes.

**How to apply.** Do not add turn logic, turn phases or grid-cell movement to Long Way. Use `time.delta_secs()` for anything that changes over time. Expect NPCs, dialogue and pathfinding later, so keep rules in plain functions that moving entities share (`move_feet`). Evaluating a physics crate stays on the roadmap (item 4). Other games in the workspace choose their own timing model. This updates the rationale of `adr-lw1`, which assumed a later grid game.

## adr-lw8. Maps are markdown files, rocks come first (2026-11-01)

**Decision.**
1. A human describes object layouts in a markdown file with frontmatter, a `# Features` section (one `## Name` heading and `cells` block per feature) and a `# Map` section (bullets such as `- 'Name' at (2,12), (2,3)`, changed 2026-11-01). Cells are painted as text, one character per 64 pixel cell, in the same cell numbers as the F2 grid (origin bottom left). The exact format is in `dev-docs/PLAN-forest-collision.md` until it is built.
2. Rocks come before trees. `Rocks.aseprite` has two frames (tags `rock_0`, `rock_1`). Trees followed the same day: `green_tree` and `red_tree` are map kinds, placed from the map file with the trunk collision rule of `adr-lw1`.
3. A rock is solid on all sides. Guy walks only around the outline of a rock cluster.
4. One rock covers 2x2 cells (128 pixels). Its collision rectangle is inset by 10% on the left, right and top, and not at the bottom, so the bottom line sits on the lowest art pixel (changed 2026-11-01 after the developer saw the line cut into the sprite).
5. Rocks are drawn overlapping (shrub look). Placement is one character per cell, rocks drawn at 128 px with no jitter (variant V1, chosen on 2026-11-01 from three variants). `pitch`, `draw` and `jitter` stay as optional frontmatter keys. Collision stays on the inset footprint. The slide range on long edges is decided by experiment.
6. Map loading starts with `include_str!` and an own parser. A sturdier pipeline is roadmap item 6.
7. Map cell (0, 0) is a fixed world position, `MAP_ORIGIN` in `camera.rs` (-1024, -576), the corner of a 16:9 view rounded to whole cells. The F2 grid uses it, so cells do not move when the window is resized. This replaces the window dependent `BottomLeft` grid origin in Long Way.
8. `Rocks.aseprite` is the native 32x32 art, scaled in the engine by a whole number (`draw` divided by 32).

**Rationale.** The developer wants simple, human editable map making in the first iteration, and wants to give Claude a layout in a form that is easy to write and review. Rocks are a simpler shape than trees (no walk-behind part), so they test merged collision first.

**How to apply.** See the plan for the file format, parser and collision steps. Trees reuse the same map format later with their own object kind and collision rule.

## adr-lw9. Draw order layers (2026-11-01)

**Decision.** Everything on screen belongs to one of six layers, from back to front, defined in `games/long-way/src/layers.rs`:
1. **Background** (sprite z `BACKGROUND_Z`)
2. **Grid debug** (sprite z `GRID_DEBUG_Z`): the F2 grid
3. **World** (sprite z within `WORLD_HALF_BAND` of 0): features (rocks, trees) **and** characters (the player, NPCs), sorted together by their feet in `depth.rs`
4. **Feature debug** (sprite z `FEATURE_DEBUG_Z`): the yellow rock outline and other debug drawings of features
5. **HUD** (UI `GlobalZIndex` `HUD_Z_INDEX`): the info box and other readouts that are always on screen
6. **UI** (UI `GlobalZIndex` `UI_Z_INDEX`): dialogs and menus, above the HUD

**Rationale.** Bevy has no layer system for 2D draw order: sprites are sorted by z, equal z has no fixed order, `RenderLayers` only picks which camera sees an entity, and 2D gizmos are always drawn last, on top of everything. The developer asked for formal layers on 2026-11-01. Features and characters share one layer because a character must walk behind a tree or rock when its feet are above the feature's sort line (`adr-lw1`, `adr-lw2`). Separate layers would stop that. The debug layer is above the world, so debug lines also draw over the player.

**How to apply.** Never write a bare z number outside `layers.rs`. `bevy_ui` draws after all sprites and gizmos, so the HUD and UI are always above the feature debug layer. A background must use a z below `GRID_DEBUG_Z`. `depth.rs` has a debug assertion that feet stay inside the world band (y within about 100000). Debug drawings that must follow the layers are sprites, not gizmos. The collision overlay in `collision_debug.rs` follows this rule.

## adr-lw10. Sliding along merged outlines, into notches (2026-11-01)

**Decision.** Blocked rectangles that touch or overlap are merged into one outline per cluster (`outline.rs`). Guy collides with the outline edges only, and `allow_sliding` (`adr-lw4`) works on those edges. The slide rules for an outline are:
1. The range is the smaller of 35% of the face and 64 pixels (`SLIDE_RANGE_CAP`), measured from the nearer end.
2. An end at an inward (concave) corner never slides Guy. Past it there is only solid ground.
3. At a convex end, Guy always slides. If the way on is open for 192 pixels (`SLIDE_CLEARANCE`) he stops exactly on the end. If a face blocks it (a notch or an overhang), he stops 0.5 pixels past the end (`SLIDE_PAST_CORNER`) and walks on until that face stops him. Example: pushing up into the bottom face of the test cluster from world x 463, Guy slides to x 440.3 and walks up to y 64.
4. All blocked areas (trunks and rocks) are merged by one `outline::build`, so a trunk touching a rock is one cluster. Two rectangles that touch only at a corner leave no gap: that vertex counts as an inward corner. A slide target is passed through `resolve`, so it never ends inside another shape.
5. A rock's blocked area is the footprint inset by 10% on the left, right and top, not at the bottom (`adr-lw8`), grown sideways by Guy's half width. A rock sorts at the bottom of its blocked area plus 1 pixel (`SORT_BIAS`), so Guy standing at the rock is drawn in front.

**Rationale.** The first version refused to slide when the way past the end was blocked within 192 pixels, to avoid pushing Guy into pockets. The developer tried it at a notch under an overhang and wanted him to slide to the corner and walk on, stopping at the back of the notch (2026-11-01). Sliding into an inward corner is still refused, because it only puts Guy inside the next rock.

**How to apply.** The rules are in `slide_to_nearest_edge` in `collision.rs`, with tests for pockets, notches and long faces. The numbers are first values, tuned by playing. A full description of all collision rules is in `dev-docs/BEVY-ADDITIONS.md`.
