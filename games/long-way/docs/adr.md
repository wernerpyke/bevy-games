# Long Way: game decisions

Decisions that apply to Long Way only. Decisions about reusable code (collision, draw order, debug tools, map format) are in `dev-docs/adr/EXTENSIONS.md` with the prefix `x`, and are described in `dev-docs/extensions/`. The format and ID rules are in `dev-docs/adr/README.md`.

| ID | Decision |
|---|---|
| adr-lw1 | Custom tree collision with a blocked band |
| adr-lw7 | Long Way is a real-time top-down RPG |
| adr-lw8 | Maps are markdown files, rocks come first |
| adr-lw12 | Long Way draw order layers |
| adr-lw13 | Rock blocked area and sort line |

## adr-lw1. Custom tree collision with a blocked band, no physics crate (2026-10-31)

**Decision.** Trees block Guy with a hand-written rule, not a physics engine. Guy's feet are blocked from entering a rectangle around each trunk. The rectangle is:
- **Sides:** the trunk's left and right edges, widened by Guy's half width (20), so his body cannot overlap the trunk from left or right.
- **Bottom:** the tree's base minus half of Guy's height (30). Walking up from below, Guy stops with the top half of his body over the trunk base (50% overlap).
- **Top:** the tree's base plus 10% of the tree height. Walking down from above, Guy stops before the bottom 10% of the tree. Above this line he can walk behind the tree, overlapping the trunk and canopy freely.

**Rationale.** These are the developer's rules for a top-down view, and they are not a normal box collision. A rectangle test on feet positions expresses all three rules with four numbers. The game is a real-time top-down RPG (`adr-lw7`) and these rules are not physics, so a physics engine is not wanted for now (roadmap item 3 evaluates `avian2d`).

**How to apply.** The rule lives in `games/long-way/src/collision.rs` as plain functions with unit tests, with no Bevy types except `Vec2` and `Rect`. Constants (`GUY_HALF_WIDTH`, `GUY_HEIGHT`, `OVERLAP_FROM_BELOW`, `BLOCKED_BOTTOM`) set the numbers. Trunk edges come from the art (`TRUNK_LEFT_PX` and `TRUNK_RIGHT_PX` in `trees.rs`). Each axis is resolved separately so Guy slides along a trunk, and a move is stopped when it crosses an edge, so a large step cannot jump over a trunk. The values are listed in `games/long-way/docs/collision.md`.

## adr-lw7. Long Way is a real-time top-down RPG (2026-11-01)

**Decision.** Long Way is a real-time, top-down RPG. It is not turn-based and not locked to a grid. Movement, sliding and animation use frame time. The 64 pixel cell is a tool for placing things (the F2 grid and map files), not a movement rule.

**Rationale.** The developer chose this direction on 2026-11-01. It replaces the earlier workspace default of turn-based play for this game. Walking, collision and the slide delay were already real-time, so no existing code changes.

**How to apply.** Do not add turn logic, turn phases or grid-cell movement to Long Way. Use `time.delta_secs()` for anything that changes over time. Expect NPCs, dialogue and pathfinding later, so keep rules in plain functions that moving entities share (`move_feet`). Evaluating a physics crate stays on the roadmap (item 3). Other games in the workspace choose their own timing model. This updates the rationale of `adr-lw1`, which assumed a later grid game.

## adr-lw8. Maps are markdown files, rocks come first (2026-11-01)

**Decision.**
1. A human describes object layouts in a markdown file with frontmatter, a `# Features` section (one `## Name` heading and `cells` block per feature) and a `# Map` section (bullets such as `- 'Name' at (2,12), (2,3)`, changed 2026-11-01). Cells are painted as text, one character per 64 pixel cell, in the same cell numbers as the F2 grid (origin bottom left). The format is in `dev-docs/extensions/map-files.md` and `adr-x11`.
2. Rocks come before trees. `Rocks.aseprite` has two frames (tags `rock_0`, `rock_1`). Trees followed the same day: `green_tree` and `red_tree` are map kinds, placed from the map file with the trunk collision rule of `adr-lw1`.
3. A rock is solid on all sides. Guy walks only around the outline of a rock cluster.
4. One rock covers 2x2 cells (128 pixels). Its collision rectangle is inset by 10% on the left, right and top, and not at the bottom, so the bottom line sits on the lowest art pixel (changed 2026-11-01 after the developer saw the line cut into the sprite).
5. Rocks are drawn overlapping (shrub look). Placement is one character per cell, rocks drawn at 128 px with no jitter (variant V1, chosen on 2026-11-01 from three variants). `pitch`, `draw` and `jitter` stay as optional frontmatter keys. Collision stays on the inset footprint. The slide range on long edges is decided by experiment.
6. Map loading starts with `include_str!` and an own parser. A sturdier pipeline is roadmap item 4.
7. Map cell (0, 0) is a fixed world position, `MAP_ORIGIN` in `camera.rs` (-1024, -576), the corner of a 16:9 view rounded to whole cells. The F2 grid uses it, so cells do not move when the window is resized. This replaces the window dependent `BottomLeft` grid origin in Long Way.
8. `Rocks.aseprite` is the native 32x32 art, scaled in the engine by a whole number (`draw` divided by 32).

**Rationale.** The developer wants simple, human editable map making in the first iteration, and wants to give Claude a layout in a form that is easy to write and review. Rocks are a simpler shape than trees (no walk-behind part), so they test merged collision first.

**How to apply.** The file format is in `dev-docs/extensions/map-files.md` and how to write a Long Way map is in `games/long-way/docs/maps.md`. Trees reuse the same map format with their own object kind and collision rule.

## adr-lw12. Long Way draw order layers (2026-11-01)

**Decision.** Long Way uses six layers, from back to front, defined in `games/long-way/src/layers.rs`. The mechanism is `adr-x9`.
1. **Background** (sprite z `BACKGROUND_Z`)
2. **Grid debug** (sprite z `GRID_DEBUG_Z`): the F2 grid
3. **World** (sprite z within `WORLD_HALF_BAND` of 0): features (rocks, trees) **and** characters (the player, NPCs), sorted together by their feet in `depth.rs`
4. **Feature debug** (sprite z `FEATURE_DEBUG_Z`): the yellow rock outline and other debug drawings of features
5. **HUD** (UI `GlobalZIndex` `HUD_Z_INDEX`): the info box and other readouts that are always on screen
6. **UI** (UI `GlobalZIndex` `UI_Z_INDEX`): dialogs and menus, above the HUD

**Rationale.** The developer asked for formal layers on 2026-11-01. Features and characters share one layer because a character must walk behind a tree or rock when its feet are above the feature's sort line (`adr-lw1`, `adr-x2`). The debug layer is above the world, so debug lines also draw over the player.

**How to apply.** A background must use a z below `GRID_DEBUG_Z`. `depth.rs` has a debug assertion that feet stay inside the world band (y within about 100000). The collision overlay in `collision_debug.rs` follows this rule.

## adr-lw13. Rock blocked area and sort line (2026-11-01)

**Decision.** A rock's blocked area is its footprint inset by 10% on the left, right and top, and not at the bottom (`adr-lw8`), grown sideways by Guy's half width. A rock sorts at the bottom of its blocked area plus 1 pixel (`SORT_BIAS`), so Guy standing at the rock is drawn in front.

**Rationale.** These are Long Way's rock shapes. The merged outline and the slide rules that use them are `adr-x10`. With the footprint bottom as the sort line, Guy standing at the blocked bottom was drawn behind the rock and looked as if he had walked in (found by the developer, 2026-11-01).

**How to apply.** The numbers are in `games/long-way/docs/collision.md`. Rock `draw` and `cell` sizes must agree, or the sprite and the blocked area drift apart.
