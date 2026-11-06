# Extensions: decisions

Decisions about reusable code that Long Way implements first. They were `adr-lw2` to `adr-lw11` until 2026-11-02, when the prefix changed to `x` and the numbers stayed: `adr-lw2` is now `adr-x2`. Game only decisions are in `games/long-way/docs/adr.md` (`adr-lw1`, `adr-lw7`, `adr-lw8`, `adr-lw12`, `adr-lw13`).

## adr-x2. Draw order sorted by feet (2026-10-31)

**Decision.** Entities with a `DepthSort` component get their z set from the y position of their feet: the lower the feet on screen, the closer to the camera.

**Rationale.** The player must draw behind a tree when its feet are above its base and in front when below. A fixed z cannot do both. The same sort line also decides collision, so what the player sees matches where it can walk.

**How to apply.** Give any sprite that should sort a `DepthSort { feet_offset }`, the distance from its centre down to its feet. `depth.rs` sets `z = -feet_y * 0.001`. Sprites with the same feet y are ordered by x (`z += x * 1e-8`), because equal z flickers (found with the rock rows, 2026-11-01). The sort runs in `Update` with no ordering against movement, so z can lag one frame.

## adr-x3. Collision debug overlay (2026-10-31, changed 2026-11-01)

**Decision.** A debug overlay shows the collision. First version: F1, gizmos, red trunk boxes and green feet circles. Now (`collision_debug.rs`): it follows the F3 grid switch and is drawn as sprites on the feature debug layer (`adr-x9`). Yellow lines are every outline edge (with a tick on the blocked side), cyan boxes are interior rocks, green squares are the feet of every depth-sorted entity. The red trunk boxes are gone because the yellow outline already covers trunks. F1 does nothing in the kit. R-Town uses F1 to reload its map (`adr-x19`).

**Rationale.** Collision rules are hard to check by eye, and Claude cannot watch the window.

**How to apply.** Keep the overlay while collision work continues. Remove it or put it behind a debug feature before any release build.

## adr-x4. Forgiving collision: `allow_sliding` (2026-10-31)

**Decision.** `CollisionPlugin` takes an `allow_sliding` flag. Long Way sets it to `true`. When true:
1. The player slides along a trunk when walking diagonally into it.
2. Walking straight into a face stops the player, which shows the collision.
3. If the player keeps pushing for 0.2 seconds (first 0.5, shortened on 2026-10-31 after the developer tried it) and its feet are within 35% of the face length from an edge (capped at 64 pixels, see `adr-x10`, which also covers faces of merged outlines and notches), it is slid sideways at 120 units per second to that edge, then walks on past the trunk. This works from below (slide left or right), from above, and into the trunk sides (slide up or down).
Pushing into the middle of a face never slides the player, so it must walk around.

When false, any blocked move leaves the player where it was. There is no sliding of any kind.

**Rationale.** Hitting a trunk by a few pixels should not make the player back off and line up again. The pause makes the collision visible before the game helps. Pushing into the middle is a clear signal that the player means to reach the trunk, so no help is given there.

**How to apply.** `SLIDE_DELAY`, `SLIDE_SPEED` and `SLIDE_RANGE_FRACTION` in `collision.rs` set the numbers. Entities that move with `move_feet` need a `SlideAssist` component, and call `reset()` on it when they are not trying to move. The setting is a resource (`CollisionSettings`) set by the plugin. Assisted sliding only starts when the input is straight along one axis. The collision rule from `adr-lw1` is unchanged. The push direction is stored as signs (for example (0, 1)), not as a normalised float, because comparing floats made the timer reset every few frames (`adr-x5`).

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

## adr-x9. Draw order layers (2026-11-01)

**Decision.** Everything on screen belongs to a named layer, from back to front, defined in the kit's `layers.rs` (`crates/game-kit`), which a game may override or extend (`adr-x14`, changed 2026-11-02; before this each game had its own file). Sprite layers are bands of z. HUD and UI layers use `GlobalZIndex`. Features and characters share one world layer, sorted together by their feet (`depth.rs`). Debug drawings have their own layers above the world. The layers Long Way uses are in `adr-lw12` (`games/long-way/docs/adr.md`).

**Rationale.** Bevy has no layer system for 2D draw order: sprites are sorted by z, equal z has no fixed order, `RenderLayers` only picks which camera sees an entity, and 2D gizmos are always drawn last, on top of everything. The developer asked for formal layers on 2026-11-01. Features and characters share one layer because a character must walk behind a tree or rock when its feet are above the feature's sort line (`adr-lw1`, `adr-x2`). Separate layers would stop that. The debug layer is above the world, so debug lines also draw over the player.

**How to apply.** Never write a bare z number outside the kit's `layers.rs` or the game's own layers module (`adr-x14`). `bevy_ui` draws after all sprites and gizmos, so the HUD and UI are always above the feature debug layer. A background must use a z below `GRID_DEBUG_Z`. `depth.rs` has a debug assertion that feet stay inside the world band (y within about 100000). Debug drawings that must follow the layers are sprites, not gizmos. The collision overlay in `collision_debug.rs` follows this rule.

## adr-x10. Sliding along merged outlines, into notches (2026-11-01)

**Decision.** Blocked rectangles that touch or overlap are merged into one outline per cluster (`outline.rs`). The player collides with the outline edges only, and `allow_sliding` (`adr-x4`) works on those edges. The slide rules for an outline are:
1. The range is the smaller of 35% of the face and 64 pixels (`SLIDE_RANGE_CAP`), measured from the nearer end.
2. An end at an inward (concave) corner never slides the player. Past it there is only solid ground.
3. At a convex end, the player always slides. If the way on is open for 192 pixels (`SLIDE_CLEARANCE`) it stops exactly on the end. If a face blocks it (a notch or an overhang), it stops 0.5 pixels past the end (`SLIDE_PAST_CORNER`) and walks on until that face stops it. Example: pushing up into the bottom face of the test cluster from world x 463, the player slides to x 440.3 and walks up to y 64.
4. All blocked areas (trunks and rocks) are merged by one `outline::build`, so a trunk touching a rock is one cluster. Two rectangles that touch only at a corner leave no gap: that vertex counts as an inward corner. A slide target is passed through `resolve`, so it never ends inside another shape.

The blocked area and sort line of a rock are game rules, in `adr-lw13` (`games/long-way/docs/adr.md`).

**Rationale.** The first version refused to slide when the way past the end was blocked within 192 pixels, to avoid pushing the player into pockets. The developer tried it at a notch under an overhang and wanted the player to slide to the corner and walk on, stopping at the back of the notch (2026-11-01). Sliding into an inward corner is still refused, because it only puts the player inside the next rock.

**How to apply.** The rules are in `slide_to_nearest_edge` in `collision.rs`, with tests for pockets, notches and long faces. The numbers were accepted by the developer after play testing (2026-11-02). The full rules are in `dev-docs/extensions/collision.md`, the Long Way values in `games/long-way/docs/collision.md`.

## adr-x11. Map file layout: Features and Map sections (2026-11-01)

**Superseded by `adr-x18` (2026-11-02): a map is now one drawn grid. This record keeps the reasons it gave.**

**Decision.** A map file is markdown with frontmatter and two sections the game reads (`dev-docs/extensions/map-files.md`):
1. `# Features`: each `## Name` heading defines a feature with exactly one `cells` block. The bottom left character is the anchor.
2. `# Map`: a bulleted list that places features, `- 'Name' at (x,y), (x,y) and (x,y)`. Positions are the anchor cell, in F3 grid numbers.
Everything else in the file is notes for people. Trees are map kinds (`green_tree`, `red_tree`) placed the same way as rocks. Every format error stops loading and names the file and line.

**Rationale.** The first format put `at=x,y` on each `cells` block, with a separate `name=` and `place` block for reuse. The developer wanted features described once, with comments, and placement in one readable list. A feature used several times is one bullet. It also lets the same file place trees, which need no change to the format, only a new kind in code.

**How to apply.** The parser is `parse` in `map.rs` (plain Rust, unit tested). Add a new object kind in `Kind` and in the module that spawns it (`rocks.rs`, `trees.rs`), and name it in the legend. Do not add per-block options to `cells`. Roadmap item 5 considers other formats (YAML, TOML) if maps outgrow markdown. Roadmap item 5 covers a sturdier loader.

## adr-x12. Shared code lives in one kit crate, `crates/game-kit` (2026-11-02)

**Decision.** Code that more than one game uses moves into the library crate `crates/game-kit`. One crate, not several. Games depend on it through `[workspace.dependencies]`. The kit has no assets and no game values.

**Rationale.** R-Town is the second game (adr-w6). One crate means one manifest and one import path. Splitting into small crates can happen later if compile times or dependencies demand it.

**How to apply.** A game supplies its values through plugin fields and marker components, not generics or Cargo features (`dev-docs/extensions/plugin-configuration.md`). Ask before adding game specific code to the kit.

## adr-x13. Map object kinds are strings in the kit (2026-11-02)

**Decision.** The map parser returns each object's kind as the legend name (`String`). Each game converts names to its own enum.

**Rationale.** The kit must not change when a game adds an object. A generic parser over the game's enum was rejected: it spreads generics through the parser and plugin for little gain. A shared enum was rejected: every new object in any game would edit the kit.

**How to apply.** An unknown name is reported by the game's conversion with the map file line, as the parser reports it today.

## adr-x14. Kit holds default layers, games override (2026-11-02)

**Decision.** The kit has a `layers` module with the default z values. Plugins take the values from plugin fields that default to those constants. A game overrides a value or adds its own layers in its own module. This changes adr-x9, which said every z number is in one file per game.

**Rationale.** Both games start with the same draw order without copying it. A game that needs another layer can add it.

**How to apply.** Still never write a bare z number in game code: use a name from the kit or the game's layers module. Keep the compile-time ordering checks in the kit. A game that overrides a value must keep the checks true.

## adr-x15. Scripted run tool is in the kit and extensible (2026-11-02)

**Decision.** The kit owns the script parser and runner (teleport, hold, wait, screenshot, quit). The game marks its target entity with a `ScriptTarget` component and can register its own commands.

**Rationale.** Both games get scripted runs and screenshots without copying (adr-x5). R-Town will need game commands such as attacks once combat exists.

**How to apply.** Game commands are added through a registration API in the kit. Design that API with the first game command, not before.

## adr-x16. Collision blockers are a kit component, slide values scale with the cell size (2026-11-02)

**Decision.** The kit has a `Blocker(Rect)` component. A game grows each blocked area by the actor's size and spawns it as a `Blocker`, or calls `CollisionWorld::add_blocked`. The slide values are `SlideTuning` fields, and `CollisionPlugin { allow_sliding, cell_size }` computes them with `SlideTuning::for_cell_size`, which scales speed, range cap, clearance and past-corner from the values tuned at a cell size of 64.

**Rationale.** The kit does not need to know a game's obstacle types (Long Way `Trunk`, R-Town houses). The actor size is already applied when a game builds the area, so `resolve` and `move_feet` need no size parameter. Long Way at cell size 64 keeps its exact values. R-Town at 16 gets a quarter, so the slide feels the same relative to the art.

**How to apply.** Keep game rules (which part of a tree blocks, actor size) in the game. Keep the kit free of game types.

Refinement (2026-11-02): `parse` takes the list of kind names the game knows and reports an unknown legend name itself, with the file line, as before. The game's conversion from name to enum then cannot fail. This keeps the parser error messages and their tests in the kit. The `draw` key is optional (`Option<f32>`) and the game gives the default.

## adr-x17. Camera follow in the kit, debug sizes scale with the cell size (2026-11-02)

**Decision.** `CameraFollowPlugin` moves the `Camera2d` to the `Controlled` entity and stops it at `CameraBounds` (a resource a game inserts once the map size is known). The camera position is rounded to a whole world unit. The debug grid and the collision overlay take their line thickness, tick length, label size and feet marker size from base values tuned for a cell size of 64, multiplied by `DebugSettings::scale` (cell size over 64).

**Rationale.** A scrolling camera is needed by every game with a map larger than the view, and rounding keeps pixel art from shimmering when it scrolls. The debug sizes were world units chosen for Long Way's 1200 unit view. At R-Town's 240 unit view they were five times too large on screen.

**How to apply.** Insert `CameraBounds` in the plugin that loads the map. Do not move the camera elsewhere, or the follow system overwrites it each frame.

## adr-x18. Map files are one drawn grid in a text file (2026-11-02)

**Decision.** A map file is plain text with front matter and a grid (`dev-docs/extensions/map-files.md`). It replaces the markdown format of `adr-x11`.
1. The grid shows every cell of the map. One character is one cell. A legend in the front matter maps characters to object kinds. A blank or `.` is empty.
2. Each grid row is `<label>|<cells>|`, and cell k is the character at offset 2k. Rulers help a reader. Row labels are zero-based and checked. Both `.` and blanks may draw a footprint, and the parser treats them the same.
3. A character marks the anchor cell (bottom left) of an object. The footprint size belongs to the game. There are no layers, and one cell holds one object.
4. `size: WxH` is in the file. `origin` is `bottom-left` or `top-left` and only sets how rows are labelled. `parse` always returns cells numbered from the bottom left, with y up.
5. A legend entry may be a marker (R-Town `"@": player_start`). It is an ordinary object that the game reads and does not draw.
6. The F3 grid can number rows from the top (`DebugSettings::rows_from_top`) so that it matches a `top-left` file.
7. Parsing stays a small hand parser in `game-kit` with no new dependency. Every error names the file and line.

**Rationale.** The developer wanted to see a whole map in one file, edit it in a text editor, and not install an editor or tool. A drawn grid shows gaps and clusters at a glance. The markdown format split a map into features and a placement list, so no one place showed the map. Research found no standard text map format (`dev-docs/RESEARCH-map-layouts.md`), and editor formats (Tiled, LDtk, Ogmo) need their editors. A seeded generator can later return the same `Map` value as the parser, so the format does not block generated maps.

**How to apply.** Add a new kind to the game's name list and `Kind` enum and to the legend. Use fixed-width font and turn off word wrap when editing wide maps. Layers, larger objects with own settings, chunk files and a visual editor are roadmap items. The visual editor must keep this format as the source of truth.

## adr-x19. Debug keys F2 info, F3 grid, F1 map reload (2026-11-03)

**Decision.** The kit debug keys are F2 for the info box and F3 for the grid and the collision overlay (they were the other way round). (Revised by `adr-x28`: the kit now reads F1 and sends `ReloadMap`.) The collision overlay redraws when `CollisionWorld` changes, and `CollisionWorld::clear` lets a game replace its blocked areas. The script runner accepts `f1` to `f3` in `hold`, so a reload can be tested in a scripted run.

**Rationale.** The developer wanted reload first, then the info box, then the grid, in the order they use them while editing a map. A reload must redraw the overlay, or the yellow outline shows the old map.

**How to apply.** A game that reloads a map despawns what the map spawned, replaces the resources, spawns again, then calls `clear` and `add_blocked` with the new blockers.

## adr-x20. Neighbour groups in the kit (2026-11-03)

**Decision.** `crates/game-kit/src/groups.rs` finds groups of touching items on a cell grid (`neighbour_groups`, `group_sizes`, 8 neighbours or 4). A game uses the group size to choose the blocked area of an item: a thin one when it stands alone, a full one in a group. The function was moved into the kit at once, although the rule "move code to the kit when two games use it" (`adr-x12`) was not met yet, because R-Town and Long Way adopted it together.

**Rationale.** The grouping is a plain grid algorithm with no game types, so the kit can unit test it alone. Two games with the same logic would copy it. The game keeps the values: footprint sizes, bands and kinds.

**How to apply.** See `dev-docs/extensions/groups.md`.

## adr-x21. Projectiles are in the kit and collide as a ground point (2026-11-04)

**Decision.** `crates/game-kit/src/projectile.rs` has the projectile component, the plugin that moves it, the hit message and the turning of the sprite. It was written in the kit from the start, although the rule "move code to the kit when two games use it" (`adr-x12`) was not met yet, because it holds no game types and no assets and the developer asked for it. A projectile is a point on the ground that uses the same outline edges as a character's feet (`collision::resolve`). It does not have its own, exact blocked shapes.

**Rationale.** Weapons are a general need for the games in this workspace, and the movement and removal rules are plain geometry that the kit can unit test alone. Reusing the edges needs no second set of areas. The cost is that the areas are grown sideways by the shooter's feet half width, so a shot stops up to 5 px early at the side of a tree, and a lone tree stops a shot only at its base band. This matches how the player meets trees. Exact shapes can be added if play testing shows a problem.

**How to apply.** See `dev-docs/extensions/projectiles.md`.

## adr-x22. Input starts in the game and moves to the kit later (2026-11-04)

**Decision.** Input handling is a module of the game for now. R-Town has `src/input.rs` with a `PlayerInput` resource (move direction, fire) that is filled from the keyboard in `InputSet`. Gameplay systems read the resource and never `KeyCode`. The module moves to the kit when the mouse is added or a second game needs it (roadmap item 8).

**Rationale.** The resource isolates the device, so a mouse becomes a second writer of the same resource. A kit module now would have one user and a guessed API. The later move is a file move.

**How to apply.** Read `PlayerInput` with `.after(InputSet)`. The scripted run tool still works, because it writes key presses into `ButtonInput<KeyCode>` before the game reads them.

## adr-x23. Path finding on a grid built from the collision edges (2026-11-04)

**Decision.** `crates/game-kit/src/nav.rs` builds a grid of walk points (R-Town: every 4 px) and links two neighbours when a straight move crosses no outline edge from its free side. A* from the `pathfinding` crate (4.16.0, compiles for `wasm32-unknown-unknown`) searches it, and the path is smoothed to straight legs. The grid is built again when the collision areas or the map rectangle change.

**Rationale.** The grid reuses the exact outlines, thin tree bands included, so there is no second description of the obstacles and paths follow the rule the walker follows. It is easy to test without a running world. A navigation mesh crate (`bevy_northstar` and others in `adr-e4`) is heavier and tied to a Bevy version. The cost is memory and a build time of about 9 ms for a 60x60 map, once per map load.

**How to apply.** See `dev-docs/extensions/nav.md`. It is the first part of roadmap item 3.

## adr-x24. Strategies are components that write a goal (2026-11-04)

**Decision.** A movement strategy is a component with a system in `StrategySet` that writes `MoveGoal`. One kit system (`movement.rs`) turns the goal into a path and steps. The kit provides `RandomPatrol` and `Pursue`. A game adds its own strategy the same way.

**Rationale.** It is the normal ECS style and needs no trait objects or boxed state. A strategy stays small and its pure parts are unit tested. The kit does not need to know the strategies of a game. A strategy can only say "go there": a special movement needs its own mover.

**How to apply.** See `dev-docs/extensions/enemy-movement.md`.

## adr-x25. Spawner settings are legend parameters (2026-11-04)

**Decision.** A legend entry of a map file may have `key=value` parameters after its kind, for example `"S": spawn_skeleton every=10 strategy=patrol radius=120`. The parser keeps them in `MapObject.params` and the line number in `Params::line`. The game decides what they mean and reports errors with the file and the line. `Spawner` (`spawner.rs`) reads `every` and `max`.

**Rationale.** The grid keeps one character per cell, and each legend character is one configured spawner. Different intervals use different characters. The change is small and backward compatible. It is a first step into roadmap item 4 (objects with settings).

**How to apply.** See `dev-docs/extensions/spawners.md` and `map-files.md`.

## adr-x26. A seedable random number generator in the kit (2026-11-04)

**Decision.** `crates/game-kit/src/rng.rs` has `Rng` (SplitMix64) and `RngPlugin`, which adds `GameRng`. The seed is `<PREFIX>_SEED` when set, else the clock. The web build has no clock at start, so it starts from a constant and mixes in the time of the first frame.

**Rationale.** The `rand` crate needs `getrandom` and extra setup for the web build. About 60 lines with no dependency are enough. A seed that we control makes scripted runs repeatable.

**How to apply.** Take `ResMut<GameRng>` and call `f32`, `range` or `point_in_disc`. Set `R_TOWN_SEED=7` for a repeatable run.

## adr-x27. Hit boxes, and projectiles hit at their flight height (2026-11-04)

**Decision.** `HitBox { offset, half_size }` (`hitbox.rs`) is a body rectangle relative to the ground point, and it is not the feet box. A projectile hits an entity with `ProjectileTarget` and a `HitBox`, tested along its path (not only at its end point) and at its flight height (`Projectile::height`), and it stops at a blocked area first. The hit is `ProjectileHit` with `HitCause::Target(entity)`, and the game removes the target.

**Rationale.** A body sits above the feet. A shot that flies at hand height along the ground line of the shooter hits a body that stands on the same line. Testing the ground line would miss it, as the first scripted run showed. Testing the path stops a long step from jumping over a thin target.

**How to apply.** See `dev-docs/extensions/projectiles.md` and `enemy-movement.md`.

## adr-x28. F1 sends `ReloadMap` from the kit, and `--debug` turns the debug tools on (2026-11-04)

**Decision.** `game_kit::reload::ReloadPlugin` reads F1 and sends a `ReloadMap` message. The game reads it (R-Town: `reload_map.run_if(on_message::<ReloadMap>)`) and decides what a reload resets. The kit does not reload anything itself. The debug tools (F2 info box, F3 grid, collision and movement overlays) are off unless the run starts with `--debug`: `rust.sh run|script <game> --debug` sets `GAME_KIT_DEBUG=1`, and `DebugPlugin::start_visible` reads it. Without the flag the overlays start hidden, and F2 and F3 still toggle them.

**Rationale.** The key belongs with the other debug keys, and a second game can reuse it without copying the binding. What a reload resets is game state (R-Town removes enemies and projectiles), so a message keeps the kit free of game types. Screens and screenshots are clean by default, and Claude asks for the overlays when it needs them. The keys keep working so the developer can still show them in a normal run. An environment variable fits the existing `<GAME>_SCRIPT` and `<GAME>_MAP` pattern and needs no argument parsing in the games.

**How to apply.** A game adds `ReloadPlugin` and a system that runs on `ReloadMap`. Long Way reloads on it too (`map.rs`: rocks, trees and the player are tagged `MapEntity`, removed and spawned again). Claude adds `--debug` when testing or taking screenshots (`CLAUDE.md`). A web build has no environment, so the tools start hidden there and F2 and F3 show them. This replaces the part of `adr-x19` that left F1 to the game.

## adr-x29. A corner contact between blocked areas is closed with a filler (2026-11-06)

**Decision.** `outline::build` adds a filler rectangle (0.5 wide, 1.0 high, `FILLER_HALF`) at every vertex where two blocked areas touch only at a corner and the other two quadrants are empty. This replaces the rule that the four edge ends at such a vertex count as concave (`adr-x4`, rule 5 of `extensions/collision.md`).

**Rationale.** With the old rule the feet could stop exactly on the vertex, where all four edges cover them and each blocks one direction, so the player could not move at all (Long Way, one-cell tree boxes on the grid). Letting the feet leave along the free boundary instead would let them squeeze through the gap in two steps. A filler makes one solid shape: the feet stop just short of the vertex and can only back away. The filler is taller than wide so that a 45 degree line (a nav diagonal) through the vertex enters it through the middle of an edge, not exactly through its corners, where the edge test passes it. R-Town widens its blocked cells sideways, so its areas overlap at corners and the filler never applies there (same clusters and edges on `test.txt`).

**How to apply.** Nothing in a game. `resolve`, `move_feet`, projectiles and nav see ordinary edges. Tests: `outline.rs`, `collision.rs`, `projectile.rs` and `nav.rs` (names with "corner contact"). A line of another exact slope through a filler corner can still pass, the same limit as at any convex corner.

## adr-x30. Exits and entries are front matter blocks, and `player_start` marks a start (2026-11-06)

**Decision.** A map file can have two more nested front matter blocks, `exits:` and `entries:`, next to `legend:`. One line is `name: key=value ...`. An exit has `area`, `to` (map id), `entry` (entry name in the target map) and `transition` (`seamless` or `scene`). An entry has `area` and an optional `facing`. An `area` is a straight strip of cells: `x0..x1,y`, `x,y0..y1` or one cell `x,y`. A range on both axes is an error. Cells are numbered like the row labels of the file. The grid marks area cells with a blank by convention, as a reading aid. The parser reads the blocks (`crates/game-kit/src/map_areas.rs`) and a second function checks the files against each other (`map_links.rs`, `validate_links`): the target map and entry exist, and an exit and its entry have the same length. A seamless exit needs an entry on the opposite edge. A start marker is `player_start`, an ordinary legend object (`"@": player_start`), and the game reads its cell. A later map may also have one, so a scripted run can start in that map.

**Rationale.** Exits are rectangles on a map edge, usually several cells wide. A block in the front matter says so in one line and keeps legend characters free for objects that block. A strip cannot be a diagonal or a block, so a wrong exit is not possible to write. The grid marks stay a reading aid because blank and `.` are already the same empty cell (rule 12 of the format). Every exit and entry has the same length so the position along the strip carries over to the other map, for scene transitions as well as seamless ones. Checking the links when the maps are registered turns a typo in a map file into an error with the file and line. `player_start` already existed in R-Town, so the games share one name for a start.

**How to apply.** Write the blocks as in `dev-docs/extensions/map-files.md`. A map without the blocks parses as before, so R-Town is unchanged. A game that registers several maps calls `validate_links`. Moving between maps (the registry, the lifecycle, the camera) is in `dev-docs/PLAN-map-transitions.md` and not built yet.

## adr-x31. A map transition is run by the kit through three game hooks (2026-11-06)

**Decision.** `crates/game-kit/src/map_transition.rs` holds the lifecycle of a scene transition. The game registers its maps (`MapSource`, read and checked once by `MapRegistry`) and gives the kit three one-shot systems (`MapHooks`): `load` (spawn one map, every entity tagged `MapId`), `rebuild` (rebuild the collision world) and `arrive` (place the controlled entity and set its facing). The kit sends a `RequestMapChange` message when the feet of the `Controlled` entity enter an exit area (it arms again when the feet leave every exit), fades a full-screen overlay out, runs `unload` (removes every `MapId` entity), `load`, `rebuild` and `arrive` in one frame, and fades in. The player keeps the same position along the strip: the offset from the low end of the exit strip is carried to the entry strip, and moves to the other axis when the strips are on different kinds of edge (a transpose, not a rotation). Exit and entry have the same length for every transition. The state is a plain resource (`MapTransition`) and not a Bevy sub state. The F2 box shows the current map id. The script tool has `goto:<map>:<entry>`.

**Rationale.** The order of load, rebuild and arrive matters, and all of it must happen while the screen is covered. Hooks run in one fixed order inside one frame, so no frame shows a half loaded map. Messages between separate systems would need system sets and sync points for the same result. The kit stays free of game types, because the game does the spawning and the facing. The game has no app states yet, and a sub state needs a parent state, so a resource carries the three phases. Keeping the position along the strip makes a wide door feel like one door. The transpose is easy to follow when editing maps: the start cell of an exit always maps to the start cell of its entry.

**How to apply.** Add `MapTransitionPlugin { sources, kinds, start, origin, fade }`, insert `MapHooks` and tag every spawned map entity with `MapId`. Stop the player with `MapTransition::locks_input`. `reload_current_map` (F1) reads the current file again and runs `load` and `rebuild`. The first map is loaded in `PostStartup` after the collision blockers are registered, so `rebuild` must clear the world first. Seamless transitions use the same lifecycle later (`dev-docs/PLAN-map-transitions.md`) and fall back to the fade until then. See `dev-docs/extensions/map-transitions.md`.

## adr-x32. A seamless exit joins the next map into the world, the player crosses a doorway (2026-11-06)

**Decision.** A `transition=seamless` exit loads the target map beside the current one and does not fade or lock the player. The new map is placed on the edge of the exit, shifted along it until the exit strip and the entry strip lie on top of each other (`seamless_origin`). The maps are never turned, so the entry must be on the opposite edge (`adr-x30`). Both maps are in `ActiveMaps`, with a `Doorway` for the link, and the game hooks run as for a scene transition (`adr-x31`). The current map is the one that holds the feet, and it changes when the player crosses the seam. A map that is not the current one is removed after it has been out of the camera view for 1 second (`UNLOAD_AFTER_SECS`). The player may leave a map only through a doorway: `ActiveMaps::walkable` gives the map areas shrunk by the body size plus a bridge over each doorway, and `clamp_to_union` keeps a move inside them. It chooses among the areas that hold the old feet and the ones that touch them, so a long step cannot jump a wall. The camera limits ease: `CameraBounds` moves towards `CameraBoundsTarget`, the box around the active maps, with a time constant of 0.35 s. A map that would overlap an active map is refused. The F3 grid is drawn once per active map with its own numbers (`DebugGrids`).

**Rationale.** The player walks on without a cut, and the two maps share one world space, so their collision, depth sort and camera work with no extra code. Collision is already in world units, so rocks and trees near the seam form clusters across it. The doorway rule keeps the map edges solid where there is no exit. Choosing the area by where the feet were stops a frame of lag from carrying the player through a wall into a map that is nearer than the one they were in. Maps that sit just outside the view are removed late so the one that was just loaded does not vanish before the camera reaches it. Easing the limits avoids a camera jump when a map joins or leaves. Labels at the top and right edge line of a map are not drawn, because the line belongs to the next map.

**How to apply.** Set `transition=seamless` on an exit whose entry is on the opposite edge and has the same length. A game uses `ActiveMaps::walkable` and `clamp_to_union` for the player instead of one map rectangle (Long Way: `move_player`). Every system that used the map rectangle must read `ActiveMaps`. `CameraBounds` is an eased box and only fits systems that use it for the camera: a game with enemies that read it for their area should not use it with seamless maps yet. The `rebuild` hook runs after every load and unload and must rebuild from all entities. Not built: an exit from the second map back to the first (the walk back works while the first map is still loaded), enemies and projectiles across a seam, and a layout of three maps around one map. `dev-docs/extensions/map-transitions.md`.
