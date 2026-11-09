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
| adr-lw19 | The player starts at the `player_start` marker |
| adr-lw21 | Long Way leaves a map by touching a marker |
| adr-lw23 | Zen Garden trees and shrubs join the groups with their own footprints |

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

**Decision.** The game region of the window is square and `VIEW_HEIGHT` is 512, so the camera shows 8 by 8 cells (the window was square, 720, until the HUD bar of `adr-lw24`). This replaces the 576 of `adr-lw16`. The player starts at the bottom middle of the map (changed by `adr-lw19`). Long Way has no unit tests for collision, placement or the bundled map. Collision rules are tested in `game-kit` (`adr-w7`: games test only their own mechanics).

**Rationale.** The developer is experimenting with map sizes. A test that fixes the size and object cells of the bundled map fails on every edit. The collision rules are general and the kit tests them (`adr-x4`).

**How to apply.** Edit the map file and its `size:` freely. Resizing the window shows more of the world along its longer side, because the view height is fixed. The map file is still checked at start, and a mistake stops the game with the file and line.

## adr-lw18. A tree in a cluster blocks one map cell, the trunk cell (2026-11-06)

**Decision.** This amends `adr-lw14`. A tree that stands alone still blocks its trunk band (the bottom 10% of the tree height). A tree in a group blocks exactly one cell: the bottom right cell of its 2x2 footprint, which holds the trunk (art x 76 to 104 of 128). The box is one cell wide and high, with no widening by the player. Rocks keep the rule of `adr-lw14`. Groups still count trees and rocks together.

**Rationale.** With the footprint rule a clustered tree blocked a whole 2x2 area, so the player could not reach the base of a neighbouring tree. A box the size of one cell lines up with the grid, so placing a map is easy, and boxes of neighbours touch, so the player cannot slip between them and get lost inside a cluster. The player walks up to the base of a tree between trunks, as with a lone tree.

**How to apply.** `tree_blocker` in `trees.rs` picks the shape. The kit takes any rectangle, so `game-kit` needs no change. A tree canvas of another size changes which cell holds the trunk.

## adr-lw19. The player starts at the `player_start` marker (2026-11-06)

**Decision.** This replaces the start at the bottom middle of `adr-lw17`. The player's feet start at the centre of the cell of the `player_start` marker (`"@": player_start`) in the map file. A map without the marker is reported and the player starts at the bottom middle. The marker has no art and blocks nothing, and `blocking_groups` does not count it. `1.txt` has it at cell (4,0).

**Rationale.** Several maps need their own starts, and a later map with a marker can be started directly for a scripted run (`adr-x30`). R-Town already works this way.

**How to apply.** Put one `@` in each map that the game can start in. Do not put it inside an exit area. Reading it is `player_start_feet` in `games/long-way/src/map.rs`.

## adr-lw20. Lanterns are 70% of the player height and block 80% of their width and half their height (2026-11-06)

**Decision.** `lantern`, `lantern_red`, `marker` and `marker_red` are map kinds (the markers share the lantern size and collision) (`features/Markers.aseprite`, scale 1, now 28x35 px, 70% of `PLAYER_BODY_HEIGHT`). The art is centred in the cell of its map character. Each lantern blocks the middle 80% of its width and the bottom 50% of its height, rounded to whole pixels (22x18) (first the full width grown by the player's half width, narrowed on 2026-11-06 because it looked too wide). The box is not widened by the player, as in `adr-lw18`. It is a `Blocker`, as a tree trunk is, and it does not count in the groups of `adr-lw14` and `adr-lw18` (its footprint is not 2x2).

**Rationale.** The source PNGs are 160x200 with a glow, not a pixel grid. Resizing to 28x35 once in the `.aseprite` file keeps the pixel art rule of a whole number scale (1). The first size was one cell (52x64) and was too large next to the player, so it was reduced on 2026-11-06.

**How to apply.** Constants are in `src/markers.rs`. The collision box is a plain rectangle, so a lantern next to a rock or tree gets its own edge and does not merge into their cluster rule.

## adr-lw21. Long Way leaves a map by touching a marker (2026-11-07)

**Decision.** In Long Way the markers `m` and `M` are the doors between maps. A legend entry such as `"m": marker exit_to=2` makes a marker lead to map 2, and the player arrives at the `@` of that map (`adr-x33`). The touch area is the blocked area of the marker plus 2 pixels (`TOUCH_MARGIN` in `markers.rs`), because the feet cannot enter the blocked area. It is then doubled in width and height around the same centre (`TOUCH_SCALE`, changed 2026-11-07) so a door is easier to reach. The blocked area is unchanged. The player keeps their facing. The maps no longer have `exits:` or `entries:` blocks, and all links are scene transitions.

**Rationale.** The developer wants a visible object to walk into, not an invisible strip on the map edge. Arriving at the start cell keeps every map to one way in.

**How to apply.** Map 1: `m` leads to map 2 and `M` to map 3. Map 2: `m` leads to map 3. Map 3 has no exit. Each map needs one `@`. Map files: `games/long-way/docs/maps.md`.

## adr-lw23. Zen Garden trees and shrubs join the groups with their own footprints (2026-11-07)

**Decision.** The five Zen Garden kinds (`green_tree_medium` 2x2, `red_tree_medium` 1x2, `red_tree_small`, `green_shrub` and `green_shrub_small` 1x1) take part in the neighbour groups of `adr-lw14`, together with rocks and trees. Each kind has its own footprint (`Kind::footprint`). In a group, each of the five blocks exactly its anchor cell, 64 by 64, which holds its trunk or base (the rule of `adr-lw18` for the trunk cell). Alone, a tree blocks its trunk foot and the bottom 10% of its canvas, and a shrub blocks a foot as wide as the red tree of its size and the bottom 10% of its drawn height.

**Rationale.** A planting such as the one in `1.txt`, columns 0 to 4, rows 1 to 4, mixes sizes and has to be one solid shape, or the player slips between the plants. The kit groups items of one size only. The game expands each item into the cells of its footprint and groups the cells with size 1, so a 2x2 object behaves as before, and a 1x1 shrub does not link to objects two cells away. This needs no kit change. The anchor cell is the trunk cell for all five, because the medium green tree has its trunk in its left cell (its canvas hangs from the bottom left), where `green_tree` has it in its right cell.

**How to apply.** A new blocking kind gets a footprint in `Kind::footprint` and is counted in `blocking_groups` (not a marker or a lantern). Rules: `games/long-way/docs/collision.md`, "Groups". Values per kind: the `art` table in `scenery.rs`.

## adr-lw24. Long Way has a HUD bar above the game view (2026-11-07)

**Decision.** The HUD is a bar above the game view, 100% of the view width and 31.25% (5/16) of the view height: 512 by 160 art pixels (`adr-x34`). The window is 576 by 756 (the game region is 576 by 576, the bar is 576 by 180, a scale of 1.125 = 9/8). It must fit a laptop screen: macOS shrinks a taller window, and the view then shows more than 8 by 8 cells (720 by 945 was tried and did not fit a 1600 pixel high display). The bar has its own background and content, and the map-transition fade does not cover it. The HUD starts with one text label, `Long Way`, in a simple Courier-like open-licence font (name and licence recorded in `art-pipeline.md`). Long Way has no title, pause or game over screen yet. The overlay mode is not built for Long Way.

**Rationale.** A bar keeps icons and text off the world, so the 8 by 8 cell view and the art scale stay as they are. The same scale as today means no art or map value changes.

**How to apply.** `camera.rs` holds the window size, the view height and the bar size. See roadmap item 13.

## adr-lw25. The HUD bar background is an Aseprite file (2026-11-08)

**Decision.** The bar picture is `assets/art/backgrounds/hud.aseprite`, 512 by 160 art pixels, drawn as an `ImageNode` (stretch mode) with `AseAnimation` (`bevy_aseprite_ultra`) on its own absolute, full-size child of the `Hud` root, pinned with `left` and `top` 0, because the root has padding. The game does this, not `game-kit`.

**Rationale.** The kit has no assets and no dependency on the Aseprite plugin (`adr-x12`). Two components on the `Hud` node need no kit change. The file is the bar size, so `UiScale` shows it at one art pixel per UI pixel and the art is not resampled.

**How to apply.** Edit the `.aseprite` file only. If the bar ratio changes (`HUD_BAR_RATIO`), resize the file to the new bar size. Text and widgets go on top as children of `Hud` (`hud.rs`).

## adr-lw26. Long Way maps have a dark ground with random grass and flower tiles (2026-11-08)

**Decision.** Every map has a `background:` block (`adr-x36`) with `mode: random`, the six images of `art/backgrounds/ground_tiles.aseprite` (`grass_0` to `grass_3`, `flowers_0`, `flowers_1`), `fill: 0.2`, `jitter: 16` and `scale: 2`. One image is drawn at 2x (32 pixels of art, 64 pixels) on its cell. The ground colour is `#292829` (a game palette colour). The developer tunes map 1 (`fill: 0.4`, `scale: 0.5`) as a test, and works on the shadows of the sprites themselves.

**Rationale.** A flat clear colour made the map look empty. Small loose images over a dark ground make it organic without a tile editor. The images are sparse, so the fill rate is low. Jitter and scale let the developer tune the look from the map file.

**How to apply.** Values and tile names: `games/long-way/docs/maps.md`, "Backgrounds". The art file is recorded in `art-pipeline.md`.

## adr-lw27. Four energy pickups fill a counted inventory (2026-11-08)

**Decision.** The map kinds `energy_blue`, `energy_red`, `energy_violet` and `energy_black` (legend digits `1` to `4`, `adr-x37`) are pickups drawn from `pickups/pickups.aseprite` at a quarter of its art size (16 by 24 pixels). Touching the touch box of a pickup (52 by 64) collects it (`adr-x38`). The `Inventory` resource keeps one count per colour, and the HUD shows each count next to a still icon of its colour (frame 4 of the pickup tag, `ICON_FRAME`, not animated), drawn at the size of the pickup in the world (16 by 24). Energy has no use and no cap yet.

**Rationale.** The design only says the four colours are four kinds of energy, so the smallest useful state is a count each. The pickup art is 64 by 96 and would be taller than the player at full size, so it is drawn small. The touch box is 52 wide (the touch width of an exit marker) and one cell high, centred on the base of the pickup. A smaller box made pickups hard to collect: a player walking past in front of one, at 16 pixels below its base, missed it. The pickups start their pulse at a random frame (`adr-x39`), so a row of them is not in step. The icons reuse the pickup art, so the HUD needs no new file. A still frame keeps the HUD calm, and the world pickups carry the motion.

**How to apply.** Values and kinds: `games/long-way/docs/maps.md`. Code: `pickups.rs`, `inventory.rs`, `hud.rs`.

## adr-lw28. The player has 3 HP, asked for with a message, shown as a row of icons (2026-11-09)

**Decision.** The player entity has `Hp { current, max }` (3 and 3 at the start, `hp.rs`). Other systems send `DamagePlayer { amount }` and never change `Hp`. Points never go below 0. At 0 the player stays alive: there is no game over screen yet (roadmap item 13), so the game logs it and damage stops. There is no invulnerability time. The HUD (`hud.rs`) draws one icon of `ui/hp.aseprite` (32 by 32, tags `Full` and `Empty`) for each point of `max`, in a row below the energy row, with the lost points as `Empty`. It builds the row again when `Hp` changes.

**Rationale.** A message keeps the rule (no negative HP, no damage at 0) in one place, and the enemy code needs no knowledge of the player. A row that follows `max` needs no HUD change when max HP changes later. Icons are used and not a count because a lost point reads at a glance, and 3 points fit the bar. The game does not die at 0 yet because nothing can show or restart a game over, and a dead end would block testing the ghost.

**How to apply.** Code: `hp.rs`, `hud.rs`, `player.rs` (adds `Hp`). Art: `art-pipeline.md` and `art-tags.md`. The ghost sends the message when it touches the player (`dev-docs/PLAN-ghost-enemy.md`).
