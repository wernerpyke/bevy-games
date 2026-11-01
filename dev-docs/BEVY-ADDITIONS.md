# Bevy additions

Our own reusable design decisions, built on top of Bevy. These are not part of Bevy. They are choices we made, often non-strict or game-feel driven, that other games in this workspace may want to reuse.

- How Bevy itself works: `dev-docs/BEVY-CONCEPTS.md`.
- Why a decision was made, and its history: `dev-docs/adr/`.
- This file: what each addition is, how to use it, and where it lives. Keep entries short and link to the ADR for the reasoning.

Status key:
- **In use** means a game uses it today.
- **Shared?** says whether the code is already reusable. Everything currently lives inside `games/long-way/src/`. Extraction into shared code is a roadmap item and needs the developer's approval first (`adr-w6`).

## Index

| Addition                                                                              | Where                                    | Status            |
| ------------------------------------------------------------------------------------- | ---------------------------------------- | ----------------- |
| [Forgiving collision (`allow_sliding`)](#forgiving-collision-allow_sliding)           | `collision.rs`                           | In use (Long Way) |
| [Feet-based collision zones](#feet-based-collision-zones)                             | `collision.rs`                           | In use (Long Way) |
| [Depth sorting by feet](#depth-sorting-by-feet)                                       | `depth.rs`                               | In use (Long Way) |
| [Configurable plugins](#configurable-plugins)                                         | `collision.rs`, `main.rs`                | In use (Long Way) |
| [Collision debug overlay](#collision-debug-overlay)                                                 | `collision_debug.rs`                           | In use (Long Way) |
| [Rules as plain functions with unit tests](#rules-as-plain-functions-with-unit-tests) | `collision.rs`                           | In use (Long Way) |
| [View height sets on-screen scale](#view-height-sets-on-screen-scale)                 | `camera.rs`                              | In use (Long Way) |
| [Animation tag switching](#animation-tag-switching)                                   | `player.rs`                              | In use (Long Way) |
| [Scripted input](#scripted-input)                                                     | `script.rs`, `dev-scripts/rust.sh`       | In use (Long Way) |
| [Logging strategy](#logging-strategy)                                                 | `collision.rs`                           | In use (Long Way) |
| [Debug grid and info box](#debug-grid-and-info-box)                                   | `debug.rs`                               | In use (Long Way) |
| [Draw order layers](#draw-order-layers)                                               | `layers.rs`                              | In use (Long Way) |
| [Markdown map files](#markdown-map-files) | `map.rs`, `rocks.rs`, `trees.rs` | In use (Long Way) |
| [Collision and sliding rules](#collision-and-sliding-rules)                           | `collision.rs`, `outline.rs`, `rocks.rs` | In use (Long Way) |

## Forgiving collision (`allow_sliding`)

**What.** A setting that makes collision friendly to the player. When `allow_sliding` is true:
1. Walking diagonally into an obstacle slides along it.
2. Walking straight into an obstacle stops the player. This shows the collision.
3. If the player keeps pushing for 0.2 seconds and is close to an edge of the obstacle face, the player is slid sideways past that edge and carries on.

Pushing into the middle of a face never slides the player. When `allow_sliding` is false, any blocked move stops the player completely.

**Why.** A near miss should not make the player back off and line up again. The pause makes the collision readable before the game helps. See `adr-lw4`.

**How to use.**
- Add the plugin with the flag: `CollisionPlugin { allow_sliding: true }`.
- Put a `SlideAssist` component on any entity that moves with `move_feet`.
- Move with `move_feet(&settings, &mut assist, old_feet, step, dt, trunks.iter())`.
- Call `assist.reset()` on frames where the entity is not trying to move.
- The push direction is compared as signs (up is (0, 1)), not as normalised floats. Float comparison made the 0.2 second timer reset every few frames.

**Tuning.** Constants in `collision.rs`: `SLIDE_DELAY` (0.2 s), `SLIDE_SPEED` (120 units per second), `SLIDE_RANGE_FRACTION` (0.35 of the face length from an edge). The delay was changed from 0.5 s to 0.2 s after the developer tried it. The other values are guesses and were not tested by hand.

**Limits.** Assisted sliding only starts when input is straight along one axis. It works from all four sides of an obstacle.

## Feet-based collision zones

**What.** Collision is tested on an entity's feet, not the centre of its sprite. Each obstacle (`Trunk`) has a blocked rectangle that the feet may not enter. The rectangle is built from three rules:
- Sides: the trunk edges widened by half the walker's body width, so bodies cannot overlap the trunk from left or right.
- Bottom: the tree base minus 50% of the walker's height, so walking up from below the walker may overlap the trunk by half its body.
- Top: the tree base plus 10% of the tree height, so walking down from above the walker can pass behind the tree but not into its bottom 10%.

**Why.** In a top-down view with a tall tree, the trunk base is what blocks movement. The canopy is only decoration the player walks behind. See `adr-lw1`.

**How to use.** Spawn a `Trunk { left, right, base_y, height }` (world coordinates) on each tree. Constants `GUY_HALF_WIDTH`, `GUY_HEIGHT`, `OVERLAP_FROM_BELOW` and `BLOCKED_BOTTOM` set the numbers. Trunk edges come from art pixels (see `trees.rs`).

**Limits.** Guy's size is a constant for now, not read from a component. Trunks and rocks are the obstacle types (see "Collision and sliding rules" below).

## Depth sorting by feet

**What.** A `DepthSort { feet_offset }` component. A system sets `z = -feet_y * 0.001`, so sprites whose feet are lower on screen draw in front.

**Why.** Bevy does not sort 2D sprites by y. A fixed z cannot let a character walk behind a tree above it and in front of it below. The same feet line decides collision, so what is drawn matches where the player can walk. See `adr-lw2`.

**How to use.** Add `DepthSort { feet_offset }` to any sprite that should sort. `feet_offset` is the distance from the sprite centre down to its feet. Add `DepthPlugin`.

**Limits.** The sort runs in `Update` without ordering against movement, so z can lag one frame.

## Configurable plugins

**What.** A game configures shared behaviour through plugin fields: `CollisionPlugin { allow_sliding: true }`. The plugin's `build` stores the value as a resource (`CollisionSettings`), and systems read it with `Res<CollisionSettings>`.

**Why.** The same code can behave differently in each game without copies or feature flags. This is the pattern to use when a shared plugin needs per-game choices.

**How to use.** Make the plugin a struct with public fields. In `build`, `insert_resource` the settings. Systems and helper functions take the settings resource (or reference).

## Collision debug overlay

**What.** The F2 switch also shows the collision: yellow outline edges with a tick on the blocked side, cyan boxes around interior rocks, green squares at the feet of every depth-sorted entity. All are sprites on the feature debug layer. See `adr-lw3`.

**Why.** Collision rules are hard to judge by eye, and Claude cannot watch the window.

**How to use.** Add `CollisionDebugPlugin`. It draws once in `PostStartup`, after every blocked area is registered, so areas added later are not shown. Feet markers follow any entity that has `DepthSort`.

**Limits.** Remove it or put it behind a debug feature before a release build. The outline is static: it does not change if areas are added or removed after startup.

## Debug grid and info box

**What.** F2 toggles a red grid with one line per grid cell (64), numbered along the top and left edges, drawn at z -10 (below sprites, above a future background). The origin is configurable (`GridOrigin`). F3 toggles a text box at the top left. See `adr-lw6`.

**Why.** Placing trees and checking collision needs a visible grid and live values.

**How to use.** `DebugPlugin { cell_size, grid_color, grid_origin }` in `main.rs`. To show a value, a system takes `ResMut<DebugInfo>` and calls `info.set("label", value)`. Add new tools as new systems in `debug.rs`.

**Limits.** Not seen on screen by Claude. Remove or gate behind a debug feature before a release build.

## Draw order layers

**What.** One file, `layers.rs`, names every layer from background to UI, with its z or `GlobalZIndex`. World sprites (features and characters) share one y-sorted layer. See `adr-lw9`.

**Why.** Bevy has no 2D layer system, and bare z numbers scattered over the code led to the grid, rock and flicker problems.

**How to use.** Import the constant from `layers`. Put a debug drawing on `FEATURE_DEBUG_Z` as a sprite, a HUD node on `GlobalZIndex(HUD_Z_INDEX)`.

**Limits.** Gizmos cannot be placed in a layer, so no debug drawing uses them.

## Collision and sliding rules

**What.** The full set of rules for what blocks Guy and how he is helped past obstacles. This is the specification the code in `collision.rs`, `outline.rs`, `rocks.rs`, `depth.rs` and `layers.rs` must follow. Each rule names its ADR.

**Terms.** Feet: the point (x, y) at the bottom of Guy's sprite, 30 below the canvas centre. Blocked area: a rectangle the feet may not enter. Cluster: blocked areas that touch or overlap, merged into one shape. Outline: the straight edges around a cluster. Face: one edge. Convex end: a face end where the shape turns away (the outside corner of the shape). Concave end: a face end where the shape turns inward (an inside corner).

### A. What is blocked

1. Only feet are tested. Nothing tests the sprite body (`adr-lw1`).
2. Trunk: blocked area from `left - 20` to `right + 20` in x (Guy's half width), and from `base_y - 30` (half Guy's height) to `base_y + 10%` of the tree height in y. Above that line Guy walks behind the tree (`adr-lw1`).
3. Rock: footprint 2x2 cells (128). Blocked area is the footprint inset by 10% on the left, right and top, no inset at the bottom, then grown sideways by 20. There is no other vertical margin (`adr-lw8`, `adr-lw10`).
4. Rocks drawn overlapping does not change collision. Blocked areas merge when they touch or overlap. With 20 pixels of sideways growth, rocks 2 cells apart join (`adr-lw8`).

### B. Outline

5. All blocked areas, trunks and rocks together, go through one `outline::build` inside `CollisionWorld::add_blocked`, which merges overlapping rectangles into clusters (coordinate compression). A trunk that overlaps a rock joins the rock's cluster. Each cluster has edges. An edge knows its side (`blocked_positive`) and whether each end is concave (`adr-lw10`). Rocks mark themselves interior in `PostStartup`, after the trunks are added.
6. A cluster member is interior when no cell it covers has an empty 4-neighbour. The flag is information only. Interior members must stay in the input of `build`, because one can fill a hole. Collision uses edges only, so interior members cost nothing per frame.
7. Edge ends: a convex end is excluded from the face (`covers`), so feet on a corner line can walk along the outside. A concave end is included, because past it the feet would be inside the shape. A vertex where two rectangles touch only at a corner counts as concave for all four edge ends there, so there is no zero-width gap to walk through. Positions within `TOLERANCE` (0.01) of an end count as at the end.

### C. Moving

8. `resolve` moves x first, then y. Each axis stops at the first edge it crosses from the free side. A large step cannot jump over a shape because the test is a crossing, not a position check. A start within `TOLERANCE` on the blocked side counts as free (rounding noise after stopping on an edge).
9. With `allow_sliding` off, any blocked move leaves Guy where he was (`adr-lw4`).
10. With it on, a diagonal move into an obstacle slides along it, because each axis is resolved alone.

### D. Assisted sliding (`allow_sliding`, `adr-lw4`, `adr-lw10`)

11. It starts only when input is straight along one axis. Pushing is tracked as signs of the step, not floats. The timer resets when the direction changes or when Guy does not push.
12. Guy must push for 0.2 s (`SLIDE_DELAY`) before any help. The pause makes the collision visible.
13. The face is the edge that blocked the pushed axis. The range is the smaller of 35% of the face and 64 (`SLIDE_RANGE_CAP`), from the nearer end. If the nearer end is outside the range, there is no slide. The middle of a long face never slides.
14. A concave nearer end refuses the slide.
15. At a convex end, Guy slides along the face at 120 units per second. The target is the end itself if the way beyond is open for 192 (`SLIDE_CLEARANCE`). If the way is blocked, the target is 0.5 (`SLIDE_PAST_CORNER`) beyond the end, so Guy is off the face and walks on until the next face stops him. The slide target goes through `resolve`, so a slide never ends inside another shape.
16. After the slide, normal movement continues. The next stop may start a new slide.

### E. Draw order (`adr-lw2`, `adr-lw9`)

17. World sprites and characters sort together by feet: `z = -feet_y * 0.001 + x * 1e-8`. Lower feet are in front. The x term only breaks ties between equal feet y, and is far too small to override a difference in y.
18. z stays inside `WORLD_HALF_BAND` (100), checked by `debug_assert`.
19. A rock sorts at its blocked bottom plus `SORT_BIAS` (1). Guy stopped at the rock edge is drawn in front of it. Guy above the rock is behind it.
20. Layers from back to front: background, grid debug, world, feature debug, HUD, UI. Draw code takes z and `GlobalZIndex` only from `layers.rs`. Debug drawings are sprites on their layer, not gizmos.

### F. Map and rocks (`adr-lw8`)

21. Map cell (0, 0) is the fixed world position `MAP_ORIGIN` (-1024, -576). A `cells` block anchors its bottom-left character at the cell given by `at`. Rows run top to bottom.
22. Rock art is 32x32, drawn at a whole-number scale. The lowest 4 art rows are empty, so the sprite is placed with its lowest visible pixel on the footprint bottom.
23. The yellow outline (F2) is drawn from the same edges collision uses. It shows the blocked side with ticks.

**Not covered yet.** Interior rock colour in the overlay and trunk overlay on the feature layer (plan step 8). Named, reusable clusters (step 9). The player clamps Guy to the view after collision (`player.rs`), which can move his feet into a shape. Guy placed inside a shape (teleport) can walk freely inside it. The 64 and 192 numbers are first guesses, and 192 assumes a 64 pixel cell. Rock `draw` and `cell` sizes must agree, or sprite and blocked area drift apart.

**Tests.** Inline in each file. Collision tests: stops, crossing, slide range, pockets, notches, long faces, noise. Outline tests: shapes, concave flags, interior (including a random brute-force check). Rock tests: placement, blocked area, sort line.

## Markdown map files

**What.** A human edits a markdown file to lay out rocks and trees. A `# Features` section defines painted features, a `# Map` section places them with a bulleted list. Collision is built from the placed objects, so no outline is drawn by hand.

**Why.** Fast, readable level layout in the first iteration, without an editor tool. See `adr-lw8` and `adr-lw11`.

**How to use.** Write the file as in `dev-docs/GAME-MAPS.md`. A new object kind needs a `Kind` in `map.rs` and a spawning system.

**Limits.** The map is read with `include_str!`, so a change needs a rebuild. One map per game build. Another format is roadmap item 6, a loader with hot reload is item 5.

## Rules as plain functions with unit tests

**What.** Game rules live in plain Rust functions that use only `Vec2` and `Rect` from Bevy, so they run without a window or an `App`. `resolve` and `move_feet` have unit tests. Run them with `./dev-scripts/rust.sh test long-way`.

**Why.** Collision rules are easy to get subtly wrong, and Claude cannot play the game. Tests catch problems such as a large step jumping over an obstacle (found this way). This follows the architecture guidance in `CLAUDE.md`.

**How to use.** Keep decision logic in functions that take plain values and return plain values. Systems only gather inputs, call the function, and write the result back.

## View height sets on-screen scale

**What.** The camera uses a fixed visible height (`VIEW_HEIGHT = 1200` world units). Art size relative to the screen is set by that number, not by the window size or by scaling sprites. Guy is 60 pixels tall, so 1200 makes him one twentieth of the screen height.

**Why.** The developer does not want to resize art or scale sprites when drawing. See `adr-e7`.

**How to use.** Change `VIEW_HEIGHT` in `camera.rs`. Smaller windows downscale the art (a 720 high window gives 0.6x), which can make pixels uneven.

## Animation tag switching

**What.** A character state (idle, walking, facing) maps to an Aseprite tag name. The animation is switched with `play_loop` only when the tag changes, and the playback speed is set at the same time (`WALK_ANIMATION_SPEED = 2.5` for walking, 1.0 for idle).

**Why.** Calling `play_loop` every frame restarts the animation. Tag names in Aseprite are the contract between art and code.

**How to use.** Keep the current tag in a field on the character component, compare before switching, and set `ase.animation.speed` in the same branch.

## Scripted input

**What.** A debug plugin that plays key presses into the running game and logs the player's feet position. It is switched on by an environment variable (`LONG_WAY_SCRIPT`) and does nothing without it. Commands, separated by `;`:
- `teleport:<feet_x>:<feet_y>`
- `hold:<keys>:<seconds>`, for example `hold:up:3` or `hold:up+left:2`
- `wait:<seconds>`
- `screenshot:<name>`, saves the window to `target/screenshots/<name>.png` (folder set by `LONG_WAY_SCREENSHOT_DIR`). Claude opens the PNG to check what is drawn. Add a `wait` before it after a `teleport`.
- `quit`

**Why.** Claude cannot play the game, and unit tests do not reproduce real frame times. A script reproduces a problem in the real app. See `adr-lw5`.

**How to use.** `./dev-scripts/rust.sh script long-way "teleport:225:-150;hold:up:3;quit"` builds the game, runs it with the script and prints the filtered log. Set `LOG_FILTER` to a different grep pattern to see more. The game opens its window while the script runs.

**Limits.** Arrow keys only. It teleports the entity marked `Player`, so each game that wants it needs its own `script.rs` or a shared version. It runs in `PreUpdate` after Bevy's input systems.

## Logging strategy

**What.** Log state changes, not every frame. `debug!` for game logic transitions (for example blocked, sliding started, no longer pushing), `info!` for script progress. The module path is the filter: `RUST_LOG=long_way=debug`. State used only to log once (such as `SlideAssist.phase`) is kept next to the logic.

**Why.** Per-frame logs bury the one line that matters. Transition logs showed the timer-reset bug in `allow_sliding`. See `adr-lw5`.

**How to use.** Add a small phase value to stateful logic and log when it changes. Use `RUST_LOG=info,wgpu=error,naga=warn` to keep Bevy's own output quiet.

**Limits.** Logging is for development. Check log volume before adding any `info!` that runs every frame.

## Adding an entry

Add an entry here when we make a reusable design decision that is not plain Bevy usage. Use the same headings: What, Why, How to use, Limits. Add a row to the index. Put the history and alternatives in an ADR and link it. Plain explanations of Bevy features belong in `BEVY-CONCEPTS.md`.
