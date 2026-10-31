# Long Way: decisions

## adr-lw1. Custom tree collision with a blocked band, no physics crate (2026-10-31)

**Decision.** Trees block Guy with a hand-written rule, not a physics engine. Guy's feet are blocked from entering a rectangle around each trunk. The rectangle is:
- **Sides:** the trunk's left and right edges, widened by Guy's half width (20), so his body cannot overlap the trunk from left or right.
- **Bottom:** the tree's base minus half of Guy's height (30). Walking up from below, Guy stops with the top half of his body over the trunk base (50% overlap).
- **Top:** the tree's base plus 10% of the tree height. Walking down from above, Guy stops before the bottom 10% of the tree. Above this line he can walk behind the tree, overlapping the trunk and canopy freely.

**Rationale.** These are the developer's rules for a top-down view, and they are not a normal box collision. A rectangle test on feet positions expresses all three rules with four numbers. The game is turn-based and uses grid logic later, so a physics engine (`bevy_rapier2d`) is not wanted.

**How to apply.** The rule lives in `games/long-way/src/collision.rs` as plain functions with unit tests, with no Bevy types except `Vec2` and `Rect`. Constants (`GUY_HALF_WIDTH`, `GUY_HEIGHT`, `OVERLAP_FROM_BELOW`, `BLOCKED_BOTTOM`) set the numbers. Trunk edges come from the art (`TRUNK_LEFT_PX` and `TRUNK_RIGHT_PX` in `trees.rs`). When the game moves to a grid, expect to replace this with grid-cell blocking. Each axis is resolved separately so Guy slides along a trunk, and a move is stopped when it crosses an edge, so a large step cannot jump over a trunk.

## adr-lw2. Draw order sorted by feet (2026-10-31)

**Decision.** Entities with a `DepthSort` component get their z set from the y position of their feet: the lower the feet on screen, the closer to the camera.

**Rationale.** Guy must draw behind a tree when his feet are above its base and in front when below. A fixed z cannot do both. The same sort line also decides collision, so what the player sees matches where he can walk.

**How to apply.** Give any sprite that should sort a `DepthSort { feet_offset }`, the distance from its centre down to its feet. `depth.rs` sets `z = -feet_y * 0.001`. The sort runs in `Update` with no ordering against movement, so z can lag one frame.

## adr-lw3. F1 collision debug overlay (2026-10-31)

**Decision.** F1 toggles a debug overlay. Red rectangles show where Guy's feet may not go. Green circles mark the feet of every depth-sorted entity.

**Rationale.** Collision rules are hard to check by eye, and Claude cannot watch the window.

**How to apply.** Keep the overlay while collision work continues. Remove it or put it behind a debug feature before any release build.

## adr-lw4. Forgiving collision: `allow_sliding` (2026-10-31)

**Decision.** `CollisionPlugin` takes an `allow_sliding` flag. Long Way sets it to `true`. When true:
1. Guy slides along a trunk when walking diagonally into it.
2. Walking straight into a face stops Guy, which shows the collision.
3. If he keeps pushing for 0.2 seconds (first 0.5, shortened on 2026-10-31 after the developer tried it) and his feet are within 35% of the face length from an edge, he is slid sideways at 120 units per second to that edge, then walks on past the trunk. This works from below (slide left or right), from above, and into the trunk sides (slide up or down).
Pushing into the middle of a face never slides him, so he must walk around.

When false, any blocked move leaves Guy where he was. There is no sliding of any kind.

**Rationale.** Hitting a trunk by a few pixels should not make the player back off and line up again. The pause makes the collision visible before the game helps. Pushing into the middle is a clear signal that the player means to reach the trunk, so no help is given there.

**How to apply.** `SLIDE_DELAY`, `SLIDE_SPEED` and `SLIDE_RANGE_FRACTION` in `collision.rs` set the numbers. Entities that move with `move_feet` need a `SlideAssist` component, and call `reset()` on it when they are not trying to move. The setting is a resource (`CollisionSettings`) set by the plugin. Assisted sliding only starts when the input is straight along one axis. The collision rule from `adr-lw1` is unchanged. The push direction is stored as signs (for example (0, 1)), not as a normalised float, because comparing floats made the timer reset every few frames (`adr-lw5`).

## adr-lw5. Debug strategy: logging and scripted input (2026-10-31)

**Decision.** Behaviour is checked in two ways besides unit tests:
1. **Logging.** Game code logs state changes, not every frame, with `debug!` under the crate's module path (for example `long_way::collision`). `info!` is for script progress. Enable with `RUST_LOG=long_way=debug`.
2. **Scripted input.** If the `LONG_WAY_SCRIPT` environment variable is set, `script.rs` presses keys in the running game and logs Guy's feet position every 0.25 seconds. Commands: `teleport:<x>:<y>`, `hold:<keys>:<seconds>`, `wait:<seconds>`, `quit`. It does nothing when the variable is not set. Run it with `./dev-scripts/rust.sh script long-way "<commands>"`.

**Rationale.** Unit tests passed for `allow_sliding` but the feature did not work in the real game. Claude cannot play the game, so it needs a way to reproduce a problem in the real app, with real frame times. The scripted run reproduced it, and the log lines ("blocked" repeating) showed the cause: the push direction was compared as floats, and rounding noise between frames kept resetting the push timer. A unit test with varying frame times now covers it.

**How to apply.** When a behaviour does not work in the real game, first reproduce it with a script, then write a failing test that captures the cause, then fix it, then rerun the script. Do not report a fix as working from unit tests alone. Do not compare floats for equality to detect a change of state: compare signs or discrete values. Keep logs to transitions so a run stays readable. New game logic that needs this kind of check should log its own transitions in the same way.

