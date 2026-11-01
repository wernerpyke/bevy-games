# Bevy additions

Our own reusable design decisions, built on top of Bevy. These are not part of Bevy. They are choices we made, often non-strict or game-feel driven, that other games in this workspace may want to reuse.

- How Bevy itself works: `dev-docs/BEVY-CONCEPTS.md`.
- Why a decision was made, and its history: `dev-docs/adr/`.
- This file: what each addition is, how to use it, and where it lives. Keep entries short and link to the ADR for the reasoning.

Status key:
- **In use** means a game uses it today.
- **Shared?** says whether the code is already reusable. Everything currently lives inside `games/long-way/src/`. Extraction into shared code is a roadmap item and needs the developer's approval first (`adr-w6`).

## Index

| Addition | Where | Status |
|---|---|---|
| [Forgiving collision (`allow_sliding`)](#forgiving-collision-allow_sliding) | `collision.rs` | In use (Long Way) |
| [Feet-based collision zones](#feet-based-collision-zones) | `collision.rs` | In use (Long Way) |
| [Depth sorting by feet](#depth-sorting-by-feet) | `depth.rs` | In use (Long Way) |
| [Configurable plugins](#configurable-plugins) | `collision.rs`, `main.rs` | In use (Long Way) |
| [F1 debug overlay](#f1-debug-overlay) | `collision.rs` | In use (Long Way) |
| [Rules as plain functions with unit tests](#rules-as-plain-functions-with-unit-tests) | `collision.rs` | In use (Long Way) |
| [View height sets on-screen scale](#view-height-sets-on-screen-scale) | `camera.rs` | In use (Long Way) |
| [Animation tag switching](#animation-tag-switching) | `player.rs` | In use (Long Way) |
| [Scripted input](#scripted-input) | `script.rs`, `dev-scripts/rust.sh` | In use (Long Way) |
| [Logging strategy](#logging-strategy) | `collision.rs` | In use (Long Way) |
| [Debug grid and info box](#debug-grid-and-info-box) | `debug.rs` | In use (Long Way) |
| [Draw order layers](#draw-order-layers) | `layers.rs` | In use (Long Way) |

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

**Limits.** Guy's size is a constant for now, not read from a component. Only trunks exist as obstacle types. Rocks and forest clusters are planned in `PLAN-forest-collision.md`.

## Depth sorting by feet

**What.** A `DepthSort { feet_offset }` component. A system sets `z = -feet_y * 0.001`, so sprites whose feet are lower on screen draw in front.

**Why.** Bevy does not sort 2D sprites by y. A fixed z cannot let a character walk behind a tree above it and in front of it below. The same feet line decides collision, so what is drawn matches where the player can walk. See `adr-lw2`.

**How to use.** Add `DepthSort { feet_offset }` to any sprite that should sort. `feet_offset` is the distance from the sprite centre down to its feet. Add `DepthPlugin`.

**Limits.** The sort runs in `Update` without ordering against movement, so z can lag one frame.

## Configurable plugins

**What.** A game configures shared behaviour through plugin fields: `CollisionPlugin { allow_sliding: true }`. The plugin's `build` stores the value as a resource (`CollisionSettings`), and systems read it with `Res<CollisionSettings>`.

**Why.** The same code can behave differently in each game without copies or feature flags. This is the pattern to use when a shared plugin needs per-game choices.

**How to use.** Make the plugin a struct with public fields. In `build`, `insert_resource` the settings. Systems and helper functions take the settings resource (or reference).

## F1 debug overlay

**What.** Pressing F1 toggles drawing of the blocked areas (red rectangles) and the feet of every depth-sorted entity (green circles), using Bevy gizmos. See `adr-lw3`.

**Why.** Collision rules are hard to judge by eye, and Claude cannot watch the window.

**How to use.** `CollisionPlugin` includes it. The state lives in the `ShowCollision` resource.

**Limits.** Remove it or put it behind a debug feature before a release build. Whether gizmos draw on top of sprites has not been checked on screen.

## Debug grid and info box

**What.** F2 toggles a red grid with one line per grid cell (64), numbered along the top and left edges, drawn at z -10 (below sprites, above a future background). The origin is configurable (`GridOrigin`). F3 toggles a text box at the top left. See `adr-lw6`.

**Why.** Placing trees and checking collision needs a visible grid and live values.

**How to use.** `DebugPlugin { cell_size, grid_color, grid_origin }` in `main.rs`. To show a value, a system takes `ResMut<DebugInfo>` and calls `info.set("label", value)`. Add new tools as new systems in `debug.rs`.

**Limits.** Not seen on screen by Claude. Remove or gate behind a debug feature before a release build.

## Draw order layers

**What.** One file, `layers.rs`, names every layer from background to UI, with its z or `GlobalZIndex`. World sprites (features and characters) share one y-sorted layer. See `adr-lw9`.

**Why.** Bevy has no 2D layer system, and bare z numbers scattered over the code led to the grid, rock and flicker problems.

**How to use.** Import the constant from `layers`. Put a debug drawing on `FEATURE_DEBUG_Z` as a sprite, a HUD node on `GlobalZIndex(HUD_Z_INDEX)`.

**Limits.** Gizmos cannot be placed in a layer. The F1 overlay still uses them.

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
