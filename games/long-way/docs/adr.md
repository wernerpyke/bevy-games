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
| adr-lw17 | The view is a square of 8 by 8 cells, no game tests for collision or the bundled maps |
| adr-lw18 | A tree in a cluster blocks one map cell, the trunk cell |
| adr-lw19 | The player starts at the `player_start` marker |
| adr-lw21 | Long Way leaves a map by touching a marker |
| adr-lw23 | Zen Garden trees and shrubs join the groups with their own footprints |

## adr-lw1. Custom tree collision with a blocked band, no physics crate (2026-10-31)

**Decision.** Trees block the player with a hand-written rule, not a physics engine. The player's feet are blocked from entering a rectangle around each trunk. The rectangle is:
- **Sides:** the trunk's left and right edges, widened by the player's half width (20), so its body cannot overlap the trunk from left or right.
- **Bottom:** the tree's base. Walking up from below, the player stops with its feet on the base line, like at the bottom of a rock (`adr-lw13`). Changed 2026-11-02: before this the bottom was the base minus half of the player's height (30), with a 50% body overlap. Trees now sort at the base plus 1, as rocks do.
- **Top:** the tree's base plus 10% of the tree height. Walking down from above, the player stops before the bottom 10% of the tree. Above this line the player can walk behind the tree, overlapping the trunk and canopy freely.

**Rationale.** These are the developer's rules for a top-down view, and they are not a normal box collision. A rectangle test on feet positions expresses all three rules with four numbers. The game is a real-time top-down RPG (`adr-lw7`) and these rules are not physics, so a physics engine is not wanted for now (`rm2` evaluates `avian2d`).

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
6. Map loading starts with `include_str!` and an own parser. A sturdier pipeline is `rm3`.
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

## adr-lw17. The view is a square of 8 by 8 cells, and the game does not test collision or its bundled maps (2026-11-06, tests updated 2026-11-11)

**Decision.** The game region of the window is square and `VIEW_HEIGHT` is 512, so the camera shows 8 by 8 cells (the window was square, 720, until the HUD bar of `adr-lw24`). This replaces the 576 of `adr-lw16`. The player starts at the bottom middle of the map (changed by `adr-lw19`). Long Way has no unit tests for collision rules or the bundled map files. Collision rules are tested in `game-kit` (`adr-w7`: games test only their own mechanics). Long Way does test its own placement: where its rocks, trees, scenery and markers stand and which area each blocks, from small maps built inside the test (updated 2026-11-11; until then the game had no placement tests either).

**Rationale.** The developer is experimenting with map sizes. A test that fixes the size and object cells of the bundled map fails on every edit. The collision rules are general and the kit tests them (`adr-x4`). Placement is Long Way's own geometry (art sizes, footprints, blocked bands), so it belongs in the game's tests, and a test map built inside the test does not change when the bundled maps do.

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

## adr-lw24. Long Way has a HUD bar above the game view (2026-11-07, amended by adr-lw31: the bar is now below, and by adr-lw34: no label)

**Decision.** The HUD is a bar above the game view, 100% of the view width and 31.25% (5/16) of the view height: 512 by 160 art pixels (`adr-x34`). The window is 576 by 756 (the game region is 576 by 576, the bar is 576 by 180, a scale of 1.125 = 9/8). It must fit a laptop screen: macOS shrinks a taller window, and the view then shows more than 8 by 8 cells (720 by 945 was tried and did not fit a 1600 pixel high display). The bar has its own background and content, and the map-transition fade does not cover it. The HUD starts with one text label, `Long Way`, in a simple Courier-like open-licence font (name and licence recorded in `art-pipeline.md`). Long Way has no title, pause or game over screen yet. The overlay mode is not built for Long Way.

**Rationale.** A bar keeps icons and text off the world, so the 8 by 8 cell view and the art scale stay as they are. The same scale as today means no art or map value changes.

**How to apply.** `camera.rs` holds the window size, the view height and the bar size. See `rm12`.

## adr-lw25. The HUD bar background is an Aseprite file (2026-11-08)

**Decision.** The bar picture is `assets/art/backgrounds/hud.aseprite`, 512 by 160 art pixels, drawn as an `ImageNode` (stretch mode) with `AseAnimation` (`bevy_aseprite_ultra`) on its own absolute, full-size child of the `Hud` root, pinned with `left` and `top` 0, because the root has padding. The game does this, not `game-kit`.

**Rationale.** The kit has no assets and no dependency on the Aseprite plugin (`adr-x12`). Two components on the `Hud` node need no kit change. The file is the bar size, so `UiScale` shows it at one art pixel per UI pixel and the art is not resampled.

**How to apply.** Edit the `.aseprite` file only. If the bar ratio changes (`HUD_BAR_RATIO`), resize the file to the new bar size. Text and widgets go on top as children of `Hud` (`hud.rs`).

## adr-lw26. Long Way maps have a dark ground with random grass and flower tiles (2026-11-08)

**Decision.** Every map has a `background:` block (`adr-x36`) with `mode: random`, the six images of `art/backgrounds/ground_tiles.aseprite` (`grass_0` to `grass_3`, `flowers_0`, `flowers_1`), `fill: 0.2`, `jitter: 16` and `scale: 2`. One image is drawn at 2x (32 pixels of art, 64 pixels) on its cell. The ground colour is `#292829` (a game palette colour). The developer tunes map 1 (`fill: 0.4`, `scale: 0.5`) as a test, and works on the shadows of the sprites themselves.

**Rationale.** A flat clear colour made the map look empty. Small loose images over a dark ground make it organic without a tile editor. The images are sparse, so the fill rate is low. Jitter and scale let the developer tune the look from the map file.

**How to apply.** Values and tile names: `games/long-way/docs/maps.md`, "Backgrounds". The art file is recorded in `art-pipeline.md`.

## adr-lw27. Four energy pickups fill a counted inventory (2026-11-08, amended by adr-lw34: at most 10 each, shown as gauges)

**Decision.** The map kinds `energy_blue`, `energy_red`, `energy_violet` and `energy_black` (legend digits `1` to `4`, `adr-x37`) are pickups drawn from `pickups/pickups.aseprite` at a quarter of its art size (16 by 24 pixels). Touching the touch box of a pickup (52 by 64) collects it (`adr-x38`). The `Inventory` resource keeps one count per colour, and the HUD shows each count next to a still icon of its colour (frame 4 of the pickup tag, `ICON_FRAME`, not animated), drawn at the size of the pickup in the world (16 by 24). Energy has no use and no cap yet.

**Rationale.** The design only says the four colours are four kinds of energy, so the smallest useful state is a count each. The pickup art is 64 by 96 and would be taller than the player at full size, so it is drawn small. The touch box is 52 wide (the touch width of an exit marker) and one cell high, centred on the base of the pickup. A smaller box made pickups hard to collect: a player walking past in front of one, at 16 pixels below its base, missed it. The pickups start their pulse at a random frame (`adr-x39`), so a row of them is not in step. The icons reuse the pickup art, so the HUD needs no new file. A still frame keeps the HUD calm, and the world pickups carry the motion.

**How to apply.** Values and kinds: `games/long-way/docs/maps.md`. Code: `pickups.rs`, `inventory.rs`, `hud.rs`.

## adr-lw28. The player has 3 HP, asked for with a message, shown as a row of icons (2026-11-09)

**Decision.** The player entity has `Hp { current, max }` (3 and 3 at the start, `hp.rs`). Other systems send `DamagePlayer { amount }` and never change `Hp`. Points never go below 0. At 0 the player stays alive: there is no game over screen yet (`rm12`), so the game logs it and damage stops. There is no invulnerability time. The HUD (`hud.rs`) draws one icon of `ui/hp.aseprite` (32 by 32, tags `Full` and `Empty`) for each point of `max`, in a row below the energy row, with the lost points as `Empty`. It builds the row again when `Hp` changes. When points are lost `hp.rs` sends `PlayerHurt`, and the player plays the `Hurt` tag of `Hunter.aseprite` once (it can walk again at once, and the walk animation waits until `Hurt` ends).

**Rationale.** A message keeps the rule (no negative HP, no damage at 0) in one place, and the enemy code needs no knowledge of the player. A row that follows `max` needs no HUD change when max HP changes later. Icons are used and not a count because a lost point reads at a glance, and 3 points fit the bar. The game does not die at 0 yet because nothing can show or restart a game over, and a dead end would block testing the ghost.

**How to apply.** Code: `hp.rs`, `hud.rs`, `player.rs` (adds `Hp`). Art: `art-pipeline.md` and `art-tags.md`. The ghost sends the message when it touches the player (`adr-lw29`).

## adr-lw29. The ghost is placed by a map marker when the map loads, patrols near its home and pursues once it sees the player (2026-11-09)

**Decision.** The legend kind `spawn_ghost` (legend character `!`, `adr-x37`) puts one ghost at the middle of its cell when the map loads. It has no timer and no respawn: unloading a map removes its ghosts (they carry the map's `MapId`) and loading it again places them again, with no state saved. The strategy comes from the legend parameters through the kit (`strategy=sentry radius=<px> sight=<px>`, `adr-x40`), and a bad entry is reported and skipped. A ghost walks at 0.3 times the player's speed. When it touches the player (`game_kit::touch`) the player loses 1 HP (`adr-lw28`), and the ghost stops, plays the `Die` tag of `ghost.aseprite` once and is removed when the animation ends. Facing is shown with the `Down`, `Left` and `Up` tags (`Right` is `Left` flipped, `adr-x41`).

**Rationale.** A marker needs no spawner: the design wants every enemy of a map to exist from the start and only once, and reloading a map is the one reset (killing is a later feature, so no state is saved yet). The parameters stay in the map file, so a map decides how far each ghost patrols and sees. A ghost that dies on touch is the whole combat rule for now, and it cannot hit twice. On touch it stops and plays `Attack` once, the player loses 1 HP when that ends, then `Die` plays (2026-11-09, the user chose this over an alert cue). The art, the walk speed, the hit box and the damage are Long Way's values. The kit has no game types (`adr-x13`).

**How to apply.** Code: `ghost.rs`, `hp.rs`, `map.rs` (`Kind::SpawnGhost`). Kinds: `games/long-way/docs/maps.md`. The art is `characters/ghost.aseprite` (`art-tags.md`).

## adr-lw30. The player moves by mouse click only, to one goal at a time (2026-11-09, amended by adr-lw38: the arrow keys walk again)

**Decision.** The player walks to the place where the left mouse button went down (`WorldClick`, `adr-x42`) and goes around blocked cells with the kit path finder (`Mover`, `adr-x43`). A new click replaces the goal while the player is walking. A click on a blocked place, or outside the walkable area, sends the player to the nearest place it can reach. There is no hold-to-follow, and the arrow keys no longer move the player. While the screen fades the click is dropped and the player stands still, and a map change clears the goal. A hit that hurts the player does not stop the walk.

**Rationale.** The developer chose click to go only, the nearest reachable point, and removing the arrow keys (2026-11-09). One movement path is less code than two, and the nearest point means a click near a tree always does something. Hold-to-follow can be added later by sending the goal again while the button is down. `Player` keeps the animation state (`walking`, `facing`, `hurt`) and reads it from `MoveState`, so the art and `Hurt` are unchanged.

**How to apply.** Code: `player.rs` (`set_goal`, `limit_player`, `facing_of`). Values: speed `WALK_SPEED` (300 px/s). `NavInset` is set from `PLAYER_HALF_WIDTH` and `PLAYER_BODY_HEIGHT` (`collision.rs`), so the ghosts, which share the grid, also do not path into those margins.

## adr-lw31. The HUD bar is below the game, and an energy icon opens a weapon drawer over the game area (2026-11-11, amended by adr-lw34: a gauge opens it, and a drawer button loads the weapon, it does not cast)

**Decision.** This amends `adr-lw24`: the HUD bar is below the game view (`ScreenMode::BarBottom`, same size, window unchanged), not above it. A click on an energy icon in the bar slides a full-width drawer up over the game area. It is 96 art pixels high (60% of the 160 pixel bar), drawn from `ui/weapon_drawer.aseprite` (like `adr-lw25`), and holds one button per weapon of that energy in one row, each with an icon (`ui/weapon_icons.aseprite`, one tag per weapon) and a text label. Red: fireball, fire ring, fire wall. Blue: ice ring, ice wall, tornado (the tornado moved first on 2026-11-14, `adr-lw34`). Violet and black: no weapons yet (the user chose this mapping). The same icon again closes the drawer, another icon switches its buttons, a click on a button sends `WeaponChosen` and the drawer slides down (0.2 s up, 0.15 s down), and a click in the game area outside the drawer closes it and still walks the player. The game does not pause, and the slide uses real time. `weapons/mod.rs` turns `WeaponChosen` into `CastWeapon` (`adr-lw32`).

**Rationale.** The weapons need a real trigger before they are built, so the player can pick one without a test variable. A drawer over the world keeps the bar small and the 8 by 8 cell view as it is. The drawer is its own UI root on the game camera, because a child of `Hud` is clipped to the HUD region. It carries `BlocksWorldClick` (`adr-x44`), or a click on a button would also walk the player. The map fade overlay now ignores picking, because it covers the whole game region and took the clicks from the drawer. Real time for the slide means a pause key added later does not freeze the drawer half way.

**How to apply.** Code: `weapon_menu.rs` (the table `weapons_for` is the energy to weapon mapping, the constants at the top hold sizes, times and colours), `hud.rs` (the clickable gauges, `adr-lw34`), `main.rs`. Art: `art-pipeline.md`, `art-tags.md`. Test it with `script long-way "ui_click:193:588;wait:0.5;screenshot:drawer;quit" --debug` (the blue gauge, `dev-feedback.md`).

## adr-lw32. Enemies have float hit points, weapons deal damage per second and push, and each weapon has its own aim rule (2026-11-11)

**Decision.** Every enemy has `EnemyHp { current, max }` (floats, `enemy_hp.rs`). Other systems ask for damage with the `DamageEnemy` message, and at 0 the enemy sends `EnemyDied` once; the ghost then plays `Die` and is removed, without hurting the player. A hurt enemy that has its own hurt frames (`HurtFrames`, an `EnemyHurt` message) plays them: the ghost plays its `Hurt` tag once (the user added it to `ghost.aseprite`, 2026-11-11). Any other enemy flashes `#993D41` for 0.12 s, by tinting the sprites of its children (the user's rule: flash unless there are hurt frames). The ghost has 100 HP (a placeholder). The player keeps `Hp`, a whole number of hearts (`adr-lw28`). Weapons are plain gameplay behind two messages: `CastWeapon { weapon, origin, facing }` (sent when the player clicks an active weapon button, `adr-lw34`) and `WeaponHit { target, weapon, amount, push }`. Each weapon has tunable values in `weapons/values.rs`: damage is either `PerSecond(dps)`, dealt in ticks of 0.5 s per target, or `Impact(n)` once (the fireball); pushback is `None`, `Pixels(n)` (at most `n` pixels in all per target per cast) or `ToEdge` (carried out to the weapon's final radius or distance). A push goes through the collision world, so trees, rocks and walls stop it, and the map edge. Aim is a rule per weapon, not a cursor: the rings start at the player's position and stay there; the walls take their orientation and travel direction from the way the player faces; the tornado surrounds the player and follows; the fireball auto-targets the nearest visible enemy. Energy cost and cooldown are not part of this (a later plan).

**Rationale.** The user chose these rules (2026-11-11): damage per second and pushback are tuned per weapon, some push a short way and some all the way, and the fireball ends on its first hit. A float keeps per-second damage exact, and a separate type keeps the HUD hearts simple. Messages keep the weapons, the damage and the enemy's death apart, like `DamagePlayer` for the player. An aim from a button has no cursor, so each weapon takes what it needs from the player. The placeholder values come from the spike or are guesses and are tuned weapon by weapon with the user.

**How to apply.** Code: `enemy_hp.rs`, `weapons/` (`mod.rs`, `values.rs`, `damage.rs`), `ghost.rs`. Values, the tuning loop and the checklist for a new weapon: `weapons.md`. Balance: `balance.md`.

## adr-lw33. The fire ring ignores features, features ignore it, and a pushed enemy stops at them (2026-11-11)

**Decision.** A ring weapon passes over trees, rocks, markers and scenery: nothing shields a body behind a feature, and the ring does not change a feature (no burn mark, no flame, no removal). An enemy that a ring pushes stops at a feature, at a wall and at the map edge, and keeps taking damage there. The ring does not hurt or react to markers such as lanterns and exits. The fire ring is centred on the middle of the player's body (`CastWeapon::centre`), does not follow the player, and is 4 cells across at full size (radius `2 * CELL_SIZE`; changed by the user on 2026-11-13, it was 6 cells across).

**Rationale.** The user chose this (2026-11-11). The hit test stays a plain circle against a body rectangle (`game_kit::area`), which is exact and cheap. Shielding would need a line-of-sight test per target against the collision outlines. A reaction that changes a feature would also have to change the collision world, so it is left for a later, visual-only step if wanted. The same rules are the starting point for the ice ring (W2).

**How to apply.** Code: `weapons/ring.rs`, `weapons/damage.rs` (`pushed_feet`). Values: `weapons/values.rs` (`RING`). Doc: `weapons.md`.

## adr-lw34. The HUD has an active weapon button and an energy gauge per energy; the drawer loads, the button casts (2026-11-12, amended 2026-11-14: blue starts with the tornado, first in its drawer)

**Decision.** This amends `adr-lw24`, `adr-lw27` and `adr-lw31`. The HUD bar keeps its background and has three bands. Top: one active weapon button per energy, from left to right red, blue, violet and black (`ui/weapon_button.aseprite`, tags `Up` and `Down`, with the weapon's icon and no label). Middle: one gauge per energy, a plain colour fill on a dark track, with no number, and a 2 pixel white line below it while the pointer is over it. Bottom: the HP icons, as before. The `Long Way` label and the energy counts are gone. Energy stops at 10 per colour, and a pickup touched at 10 is collected and lost. A click on a gauge opens the drawer of that energy. A click on a drawer button loads that weapon into the active slot of its energy and does not cast. A click on an active button casts its weapon (`CastWeapon`, `adr-lw32`) and closes the drawer if it is open. At the start, red holds the fire ring and blue the tornado, which is also the first weapon in the blue drawer (the user, 2026-11-14; it was the ice ring). A weapon that is not built yet is dimmed in the drawer and cannot be loaded, and an active button that holds one is blank. The active weapons stay when the player changes map.

**Rationale.** Play testing showed that two clicks per cast are too slow (the user's brief, 2026-11-12). Choosing a weapon is rare and casting is frequent, so the frequent action takes one click. A gauge reads at a glance where a count does not, and it needs a maximum, so 10 is a first value. Losing a pickup at the maximum needs no kit change, and energy has no use yet (cost per cast is a later plan). Dimming unbuilt weapons means a slot never holds a weapon that does nothing, while the drawer still shows what will come. The gauge colours are the main colours of the pickups (their own palette), so a gauge matches its pickup in the world.

**How to apply.** Code: `hud.rs` (bands, sizes and gauge colours at the top; the button and the gauge are the kit's `image_button` and `gauge`, `adr-x45`), `weapon_menu.rs` (`ActiveWeapons`, `default_weapon`, `WeaponKind::is_built`, the messages `GaugeClicked` and `ActiveWeaponClicked`), `weapons/mod.rs` (`cast_active`), `inventory.rs` (`MAX_ENERGY`). A weapon is turned on in `is_built` when it is built (`weapons.md`, "Adding a weapon"). Art: `art-pipeline.md`, `art-tags.md`. Script clicks are in `weapons.md`.

## adr-lw35. The ice ring is a shield: it stops at features, pushes enemies to its edge, holds them out and stuns a pinned one (2026-11-13)

**Decision.** The ice ring is a shield, not a damage weapon. It has a radius of 3 cells, grows from the middle of the player's body like the fire ring and stays where it was cast. Unlike the fire ring (`adr-lw33`), each part of it stops at the first blocking feature or the map edge, so it is not a full circle near a tree, and an enemy behind a feature is not reached. The growing front carries every enemy it reaches to its edge. Until the ring goes, no enemy can cross it; the player can. It deals a medium amount once to an enemy it pushes, and a small amount once per cast to an enemy that walks into it. An enemy that a push drives into a blocking feature or the map edge takes no more damage and is stunned until the ring goes: it does not move, its animation stops in its frame, and it does not attack. An enemy pushed into open space is not stunned. One ring never kills a ghost (balance rule b3).

**Rationale.** The user's design (2026-11-13). Stopping at features makes the ice ring feel solid, the opposite of the fire ring, which burns through everything. The player passes so that a shield never traps its caster, and the barrier stays in the game (an enemy step into the ring is undone after the kit has moved it), so the kit's collision and paths do not change. "Stunned" is the user's term, used for the kit marker too (`adr-x46`). The touch damage is once per cast so that a ghost pressing against the ring is not worn down to death, which keeps b3 simple.

**How to apply.** Code: `weapons/ice_ring.rs` (reach, push, barrier, stun), `weapons/ring.rs` (growth and look, shared with the fire ring), `weapons/values.rs` (`ICE_RING`, `ICE_RAYS`, `Damage::PushAndTouch`). Doc: `weapons.md`, `balance.md` (b3).

## adr-lw36. The tornado is a mobile shield the ghost slowly walks through; contact counts in total (2026-11-13)

**Decision.** The whirlpool is renamed the tornado. It is a disc on the middle of the player's body that follows the player and lasts twice as long as the ice ring. It deals damage per tick of contact, and contact is counted in total, so a ghost held against it dies once that passes 60% of the duration. It pushes an enemy out at a speed below a ghost's walk, and only the enemy's own steps take it deeper (`Pushback::Outward`): a ghost reaches a player who stands still, and a player who moves stays safe. It passes over features; an enemy pushed into one is held there without a stun. The look is the spike's water spiral for now. Balance rules b4 to b8.

**Rationale.** The user's intent (2026-11-13, quoted in `weapons.md`). Limiting the depth to the enemy's own steps, rather than pushing at a fixed speed, is what makes movement matter: with a plain push, a player walking at a ghost would carry the tornado's edge over it at 300 px/s and the ghost would be inside at once. Total contact lets the player kill a ghost by holding the tornado against it, which the user wants. No stun keeps the tornado distinct from the ice ring. Walking straight at one ghost for about 2.4 s lets it through; the user accepts that.

**Changed after the first play test (the user, 2026-11-13).** The push edge is where the player sees the dense droplets: it starts at their inner edge (1 cell) and grows to 1.5 cells, shown by a ring of dots that moves with the player. A weaker steady push plus a bump on every second tick knocks a ghost walking in back a couple of times (b9), so the push reads as a hit rather than a slow drift. The ghost still gets through to a player who stands still (b7).

**How to apply.** Code: `weapons/tornado.rs`, `weapons/values.rs` (`TORNADO`, `Pushback::Outward`, the tornado row). Docs: `weapons.md`, `balance.md` (b4 to b9).

## adr-lw37. The fireball is aimed by the game, steers to its target and explodes with a blast that spares the enemy it hit (2026-11-13)

**Decision.** The fireball goes for the closest living enemy on screen, or the closest on the map when none is on screen, and flies in a random direction when there is no enemy. It steers to its target every frame, so it does not miss a moving one. It explodes on the first enemy it touches, on a blocking feature, at the map edge, at the end of its range with no target, and at the last place of a target that dies or vanishes in flight. The enemy it touched takes the direct hit; every other enemy within the blast radius takes a smaller blast damage, through features. The blast does not hurt the player and does not push. Balance rules b10 to b14.

**Rationale.** The user's intent (2026-11-13, quoted in `weapons.md`): a high-power shot the player does not aim. Steering keeps the promise "does not miss" without aim prediction; at 420 px/s against a ghost's 90 it does not read as a homing missile (the user). Enemies on screen first, because the player sees them. A feature in the way is the one way to miss, as the user wants, and the blast through features still reaches an enemy close behind it. The direct target is spared the blast so that a stronger enemy that survives the direct hit is not punished twice (b14). The kit's `Projectile` already moves a shot and reports what it hits; the fireball only changes its velocity.

**How to apply.** Code: `weapons/fireball.rs`, `weapons/values.rs` (`FIREBALL`, `Damage::Impact`, the fireball row). Docs: `weapons.md`, `balance.md` (b10 to b14).

## adr-lw38. The keyboard comes back next to the mouse: arrows walk, 1 to 4 open a drawer, Z X C V cast (2026-11-13)

**Decision.** This amends `adr-lw30`. The arrow keys walk the player again, with the collision and sliding rules of `move_feet`, as before `adr-lw30`. A click still walks the player to its goal. While an arrow key is held the player has no goal: the keys win over a click in the same frame and stop a walk to a click, and the player stands when the keys are released. Two opposite keys cancel each other and do not stop a walk to a click. The keys 1, 2, 3 and 4 do what a click on the red, blue, violet and black gauge does (open, switch or close the drawer), and Z, X, C and V do what a click on the active weapon button of the same energy does (cast). They send the same messages as the clicks (`GaugeClicked`, `ActiveWeaponClicked`). A weapon in the drawer is still loaded by a click. Each key acts once per press.

**Rationale.** The user chose this control scheme (2026-11-13). Sending the click messages keeps one path from input to the drawer and the cast, so the keys and the clicks cannot drift apart. The key walk writes `MoveState` as the mover does, so the animation, `limit_player` and the aim of the weapons (the last walk direction) are the same for both walks. It has its own slide assist (`KeyAssist`), because `follow_goals` resets the mover's `SlideAssist` in every frame without a goal, and the push into a trunk would never last long enough to slide.

**How to apply.** Code: `player.rs` (`key_direction`, `stop_goal_on_keys`, `walk_by_keys`, `KeyAssist`), `hud.rs` (`WEAPON_KEYS`, `press_weapon_keys`). Test it with `script long-way "hold:left+up:0.4;hold:2:0.1;wait:0.4;hold:x:0.1;quit" --debug` (the script knows `z`, `x`, `c`, `v` and `1` to `4`, `dev-feedback.md`).

## adr-lw39. Each cast costs energy; the active button is disabled and its icon greyed until the player can pay (2026-11-14)

**Decision.** Each weapon costs energy per cast, in the energy of its drawer: fire ring 1 red, fireball 3 red, tornado 1 blue, ice ring 2 blue. The player starts with 0 of each, so pickups are the only source. The cast step takes the cost (`Inventory::try_spend` in `cast_active`); with too little energy nothing is cast and nothing is taken. While the player has less than the cost of the weapon in a slot, its active button is disabled (Bevy's `InteractionDisabled`, the `Disabled` frame of `weapon_button.aseprite`): a click and its cast key do nothing, so an open drawer stays open. A blank slot is never disabled. Over the weapon icon a layer of `#1f1932` at 50% alpha covers the missing share of the cost, from the top. The energy gauges and this overlay move at a constant speed, empty to full in 0.3 s, also when another weapon is loaded.

**Rationale.** The user's design (2026-11-14). A cost per cast makes the pickups matter and gives each weapon a price that matches its power. Taking the cost in the same step as the cast means that two casts in one frame (a click and a key) cannot both use the same energy. The cast check stays even with the disabled button, because the check is the rule and the button only shows it. The overlay reads as a gauge of the cost, and the flat disabled frame shows that a click will do nothing. Smooth gauges show a change instead of a jump. The button uses the real count, so it is enabled as soon as the energy is there, a moment before the overlay is gone.

**How to apply.** Costs: `cost` in `weapons/values.rs` (the walls have a placeholder until they are built). Spend: `inventory.rs` (`try_spend`, `cost_share`), `weapons/mod.rs` (`cast_active`). HUD: `hud.rs` (`FILL_SECS`, `OVERLAY_COLOUR`, `ShownShare`, `CostOverlay`, `update_cost_overlays`, `disable_unaffordable`, `press_weapon_keys`). Kit widgets: `adr-x45`. Art: `weapon_button.aseprite` is made with `draw_button.lua disabled=1`.
