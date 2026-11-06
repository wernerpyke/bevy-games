# Long Way: game decisions

Decisions that apply to Long Way only. Decisions about reusable code (collision, draw order, debug tools, map format) are in `dev-docs/adr/EXTENSIONS.md` with the prefix `x`, and are described in `dev-docs/extensions/`. The format and ID rules are in `dev-docs/adr/README.md`.

| ID | Decision |
|---|---|
| adr-lw1 | Custom tree collision with a blocked band |
| adr-lw7 | Long Way is a real-time top-down RPG |
| adr-lw8 | Maps are text grids, rocks come first |
| adr-lw12 | Long Way draw order layers |
| adr-lw13 | Rock blocked area and sort line |
| adr-lw14 | A lone item blocks a band, a group blocks its footprint (trees: see adr-lw18) |
| adr-lw17 | The view is a square of 8 by 8 cells, no game tests for collision |
| adr-lw18 | A tree in a cluster blocks one map cell, the trunk cell |

## adr-lw1. Custom tree collision with a blocked band, no physics crate (2026-10-31)

**Decision.** Trees block the player with a hand-written rule, not a physics engine. The player's feet are blocked from entering a rectangle around each trunk. The rectangle is:
- **Sides:** the trunk's left and right edges, widened by the player's half width (20), so its body cannot overlap the trunk from left or right.
- **Bottom:** the tree's base. Walking up from below, the player stops with its feet on the base line, like at the bottom of a rock (`adr-lw13`). Changed 2026-11-02: before this the bottom was the base minus half of the player's height (30), with a 50% body overlap. Trees now sort at the base plus 1, as rocks do.
- **Top:** the tree's base plus 10% of the tree height. Walking down from above, the player stops before the bottom 10% of the tree. Above this line the player can walk behind the tree, overlapping the trunk and canopy freely.

**Rationale.** These are the developer's rules for a top-down view, and they are not a normal box collision. A rectangle test on feet positions expresses all three rules with four numbers. The game is a real-time top-down RPG (`adr-lw7`) and these rules are not physics, so a physics engine is not wanted for now (roadmap item 2 evaluates `avian2d`).

**How to apply.** The rule lives in `games/long-way/src/collision.rs` (the trunk shape) and in `crates/game-kit/src/collision.rs` (`resolve`, `move_feet`) as plain functions with unit tests, with no Bevy types except `Vec2` and `Rect`. Constants (`PLAYER_HALF_WIDTH`, `PLAYER_HEIGHT`, `OVERLAP_FROM_BELOW`, `BLOCKED_BOTTOM`) set the numbers. Trunk edges come from the art (`TRUNK_LEFT_PX` and `TRUNK_RIGHT_PX` in `trees.rs`). Each axis is resolved separately so the player slides along a trunk, and a move is stopped when it crosses an edge, so a large step cannot jump over a trunk. The values are listed in `games/long-way/docs/collision.md`.

## adr-lw7. Long Way is a real-time top-down RPG (2026-11-01)

**Decision.** Long Way is a real-time, top-down RPG. It is not turn-based and not locked to a grid. Movement, sliding and animation use frame time. The 64 pixel cell is a tool for placing things (the F3 grid and map files), not a movement rule.

**Rationale.** The developer chose this direction on 2026-11-01. It replaces the earlier workspace default of turn-based play for this game. Walking, collision and the slide delay were already real-time, so no existing code changes.

**How to apply.** Do not add turn logic, turn phases or grid-cell movement to Long Way. Use `time.delta_secs()` for anything that changes over time. Expect NPCs, dialogue and pathfinding later, so keep rules in plain functions that moving entities share (`move_feet`). Evaluating a physics crate stays on the roadmap (item 3). Other games in the workspace choose their own timing model. This updates the rationale of `adr-lw1`, which assumed a later grid game.

## adr-lw8. Maps are text grids, rocks come first (2026-11-01, format changed 2026-11-02)

**Decision.**
1. A human draws object layouts in one text file: front matter, then a grid with one character per 64 pixel cell. Long Way uses `origin: bottom-left`, so row labels match the F3 grid. The format is in `dev-docs/extensions/map-files.md` and `adr-x18`. It replaced the markdown format with `# Features` and `# Map` sections (`adr-x11`).
2. Rocks come before trees. `Rocks.aseprite` has two frames (tags `rock_0`, `rock_1`). Trees followed the same day: `green_tree` and `red_tree` are map kinds, placed from the map file with the trunk collision rule of `adr-lw1`.
3. A rock is solid on all sides. The player walks only around the outline of a rock cluster.
4. One rock covers 2x2 cells (128 pixels). Its collision rectangle is inset by 10% on the left, right and top, and not at the bottom, so the bottom line sits on the lowest art pixel (changed 2026-11-01 after the developer saw the line cut into the sprite).
5. Rocks are drawn overlapping (shrub look). Placement is one character per cell, rocks drawn at 128 px with no jitter (variant V1, chosen on 2026-11-01 from three variants). `draw` and `jitter` stay as optional front matter keys (`pitch` went with the markdown format). Collision stays on the inset footprint. The slide range on long edges is decided by experiment.
6. Map loading starts with `include_str!` and an own parser. A sturdier pipeline is roadmap item 3.
7. Map cell (0, 0) is a fixed world position, `MAP_ORIGIN` in `camera.rs` (-1024, -576), the corner of a 16:9 view rounded to whole cells. The F3 grid uses it, so cells do not move when the window is resized. This replaces the window dependent `BottomLeft` grid origin in Long Way.
8. `Rocks.aseprite` is the native 32x32 art, scaled in the engine by a whole number (`draw` divided by 32).

**Rationale.** The developer wants simple, human editable map making in the first iteration, and wants to give Claude a layout in a form that is easy to write and review. Rocks are a simpler shape than trees (no walk-behind part), so they test merged collision first.

**How to apply.** The file format is in `dev-docs/extensions/map-files.md` and how to write a Long Way map is in `games/long-way/docs/maps.md`. Trees reuse the same map format with their own object kind and collision rule.

## adr-lw12. Long Way draw order layers (2026-11-01)

**Decision.** Long Way uses six layers, from back to front, use the kit defaults in `crates/game-kit/src/layers.rs` (`adr-x9`, `adr-x14`).
1. **Background** (sprite z `BACKGROUND_Z`)
2. **Grid debug** (sprite z `GRID_DEBUG_Z`): the F3 grid
3. **World** (sprite z within `WORLD_HALF_BAND` of 0): features (rocks, trees) **and** characters (the player, NPCs), sorted together by their feet in `depth.rs`
4. **Feature debug** (sprite z `FEATURE_DEBUG_Z`): the yellow rock outline and other debug drawings of features
5. **HUD** (UI `GlobalZIndex` `HUD_Z_INDEX`): the info box and other readouts that are always on screen
6. **UI** (UI `GlobalZIndex` `UI_Z_INDEX`): dialogs and menus, above the HUD

**Rationale.** The developer asked for formal layers on 2026-11-01. Features and characters share one layer because a character must walk behind a tree or rock when its feet are above the feature's sort line (`adr-lw1`, `adr-x2`). The debug layer is above the world, so debug lines also draw over the player.

**How to apply.** A background must use a z below `GRID_DEBUG_Z`. `depth.rs` has a debug assertion that feet stay inside the world band (y within about 100000). The collision overlay in `collision_debug.rs` follows this rule.

## adr-lw13. Rock blocked area and sort line (2026-11-01)

**Decision.** A rock's blocked area is its footprint inset by 10% on the left, right and top, and not at the bottom (`adr-lw8`), grown sideways by the player's half width. A rock sorts at the bottom of its blocked area plus 1 pixel (`SORT_BIAS`), so the player standing at the rock is drawn in front.

**Rationale.** These are Long Way's rock shapes. The merged outline and the slide rules that use them are `adr-x10`. With the footprint bottom as the sort line, the player standing at the blocked bottom was drawn behind the rock and looked as if it had walked in (found by the developer, 2026-11-01).

**How to apply.** The numbers are in `games/long-way/docs/collision.md`. Rock `draw` and `cell` sizes must agree, or the sprite and the blocked area drift apart.

## adr-lw14. A lone item blocks a band, a group blocks its footprint (2026-11-03)

**Decision.** This amends `adr-lw1` and `adr-lw13`. Footprints are 2x2 cells, so two objects touch when their anchor cells are at most 2 apart, also at a corner, and a rock and a tree can be in one group (`adr-x20`).
- A tree that stands alone blocks its trunk band, as before.
- A rock that stands alone blocks only the bottom 10% of its footprint height (`ALONE_BAND_FRACTION`), the same fraction a trunk uses. The player walks behind the rest.
- Every member of a group, tree or rock, blocks the rock footprint area of `adr-lw13`. A member with another member above it has no top inset, so the two shapes merge and no gap stays open.

**Rationale.** The same rule as R-Town (`adr-rt9`): a lone item is passable from behind and a group is one solid shape. Trunks and rocks in one group get the same shape, so the outline has no step between them. With the 10% top inset, two rocks two cells apart in a column left a 12.8 px gap.

**How to apply.** `blocking_groups` in `src/map.rs` finds the group of every object. A new blocking kind must go through it. The values are in `docs/collision.md`.

## adr-lw15. The player is the Hunter (2026-11-05)

**Decision.** The player sprite is `characters/Hunter.aseprite` (128x128, feet at y 120), from the Hunter sprite pack. It replaces `Guy.aseprite`. The feet are 56 below the canvas centre and the half width is 14 (`PLAYER_FEET_OFFSET`, `PLAYER_HALF_WIDTH`). The sprite was doubled in Aseprite on 2026-11-05 (nearest-neighbour, so each art pixel is 2x2) and is drawn at 1x. The canvas is two cells.

**Rationale.** The pack art was 22x25 px, too small next to the trees and rocks at 1x. Doubling it in the `.aseprite` file, not in the game, gives the developer more pixels to draw on and keeps the game scale at 1x. The cloak is about 20 px wide (the hat 30) after doubling, so a body-width half width would be 10. The half width is 14 because it must stay above 12.8, half of the 25.6 px gap between rocks two cells apart (`adr-x20`). A smaller value would let the player walk between rocks that the map reads as one cluster.

**How to apply.** Values and numbers are in `collision.md`. Art details are in `HUNTER-sprites.md`. Keep the half width above 12.8. If the sprite size changes again, `PLAYER_FEET_OFFSET`, `PLAYER_BODY_HEIGHT` and the half width change with it.

## adr-lw16. The camera follows the player and stops at the map edge (2026-11-05)

**Decision.** Long Way uses `CameraFollowPlugin` from the kit, as R-Town does (`adr-x17`). `VIEW_HEIGHT` is 576 (9 cells of 64), down from 1200. `CameraBounds` is the map area (`map_rect` in `camera.rs`), set when the map loads and again on F1. The player is kept inside the map, not inside the view: the feet stop at the map edge, and `PLAYER_BODY_HEIGHT` (50) below the top edge.

**Rationale.** At 1200 the Hunter was about one 24th of the screen height and hard to see. At 576 it is about one 11th, and the 32x18 map is two views wide and two high, so the camera scrolls. The camera stops at the map edge, so the body would be cut off at the top edge without the extra margin.

**How to apply.** Change `VIEW_HEIGHT` to zoom. Keep 576 divisible by the cell size so the view holds whole cells. Update `adr-e7` if the number changes again. A map smaller than the view is centred.

## adr-lw17. The view is a square of 8 by 8 cells, and the game has no collision tests (2026-11-06)

**Decision.** The window is square (`WINDOW_SIZE`, 720) and `VIEW_HEIGHT` is 512, so the camera shows 8 by 8 cells. This replaces the 576 of `adr-lw16`. The player starts at the bottom middle of the map. Long Way has no unit tests for collision, placement or the bundled map. Collision rules are tested in `game-kit` (`adr-w7`: games test only their own mechanics).

**Rationale.** The developer is experimenting with map sizes. A test that fixes the size and object cells of the bundled map fails on every edit. The collision rules are general and the kit tests them (`adr-x4`).

**How to apply.** Edit the map file and its `size:` freely. Resizing the window shows more of the world along its longer side, because the view height is fixed. The map file is still checked at start, and a mistake stops the game with the file and line.

## adr-lw18. A tree in a cluster blocks one map cell, the trunk cell (2026-11-06)

**Decision.** This amends `adr-lw14`. A tree that stands alone still blocks its trunk band (the bottom 10% of the tree height). A tree in a group blocks exactly one cell: the bottom right cell of its 2x2 footprint, which holds the trunk (art x 76 to 104 of 128). The box is one cell wide and high, with no widening by the player. Rocks keep the rule of `adr-lw14`. Groups still count trees and rocks together.

**Rationale.** With the footprint rule a clustered tree blocked a whole 2x2 area, so the player could not reach the base of a neighbouring tree. A box the size of one cell lines up with the grid, so placing a map is easy, and boxes of neighbours touch, so the player cannot slip between them and get lost inside a cluster. The player walks up to the base of a tree between trunks, as with a lone tree.

**How to apply.** `tree_blocker` in `trees.rs` picks the shape. The kit takes any rectangle, so `game-kit` needs no change. A tree canvas of another size changes which cell holds the trunk.
