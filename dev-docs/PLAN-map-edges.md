# PLAN: map edges for Long Way, with the rules in the kit

Source: `dev-docs/BRIEF-map-edges.md` (the brief). The brief holds the rules and the drawn examples. This plan holds the steps. Where the two differ, the brief wins. Fix the plan.

## Status (2026-11-15): resume here

Steps 0 to 5 done. Steps 0 to 4 are committed, step 5 is not. Next: step 6 (check in the game).

## Goal

In a map file, `.` becomes a map edge and a blank becomes free space. Each edge cell is drawn with a piece of `map_edge.aseprite` and blocks the opaque half of that piece. `game-kit` owns the rules. Long Way owns the art and the choice of edge layer.

## What is already in place

- **Parser:** `crates/game-kit/src/map.rs`. `.` is reserved and must be written `".": empty` (the check at the legend, about line 693). A blank or `.` is an empty cell (about line 573). `Map` has objects only, so it does not know which cells are `.`.
- **Areas:** `crates/game-kit/src/map_areas.rs` (exits, entries, `PLAYER_START`). Rule 21 allows `@` in an entry area and no other object in any area. No map in either game has exit or entry areas today.
- **Collision:** rectangles only (`adr-x4`, `adr-x10`). The game grows each blocked area sideways by the actor half width (Long Way: `PLAYER_HALF_WIDTH` = 14, `games/long-way/src/collision.rs`). `resolve` lets a move that starts inside a blocked area walk out (`collision.rs`, about line 102). Ghosts path with `NavGrid::build(world.edges(), ...)` (`crates/game-kit/src/nav.rs`), so edge blockers in `CollisionWorld` also block ghosts.
- **Map border:** the player's feet stay in `walkable_areas` (`games/long-way/src/player.rs`). This does not change.
- **Draw order:** `crates/game-kit/src/layers.rs` (`BACKGROUND_Z` -500, `BACKGROUND_TILES_Z` -490, `GRID_DEBUG_Z` -200, world band ±100, `FEATURE_DEBUG_Z` 200). Its `const` asserts keep the order.
- **Art:** `games/long-way/input/map_edge.aseprite`: 64×64, 9 frames, one tag per piece (`north` ... `center`). The frames have debug letters on them. Drawn examples: `input/map_edge_example.aseprite` and `input/map_edge_example_smooth.aseprite` (`input/` is git-ignored).
- **Maps:** Long Way `assets/maps/1.txt` to `3.txt` use `".": empty`. R-Town `assets/maps/test.txt` uses it too and does not change.

## Out of scope

- R-Town. It keeps `".": empty`.
- A tile layer (`rm3`, `ROADMAP.md`).
- Inside corners, except as part of the smooth corner spike (step 4).

## Decisions

Record these in the ADRs in step 7.

| Topic | Decision |
|---|---|
| Cells | `.` is an edge. A blank (`  ` or ` `) is free space. |
| Legend value | `".": edge`, a value the kit knows, like `empty`. `.` may only be `empty` or `edge`. Any other value is an error that names the line. |
| Piece | The brief's rule step 1 (free sides) and rule step 2 (a straight piece next to a different piece along its wall becomes `center`). See the brief, "Choosing the piece". |
| Not supported | A cell with exactly two free sides that are opposite (N and S, or E and W), or with four free sides: a load error that names the line. Three free sides are allowed. A `.` in an exit or entry area: a load error that names the line of the area. |
| Blocking | The opaque half of the piece. `center` blocks the whole cell. Corners block with stair-step rectangles. The step size is tuned by playing. |
| Objects | Every kind of object (features, pickups, enemy spawns, markers, `@`) follows one rule. On the outer ring, the cell is an ordinary edge cell: it gets a piece, its neighbours treat it as edge, and it blocks. In an entry area, or inside the map, the cell is free space. The object blocks as it does today. |
| Footprints | The file is the truth. A `.` footprint cell is an edge, a blank one is free. Footprints inside the map must be drawn with blanks. The kit stays without footprints (`adr-x13`). Map format rule 12 changes. |
| Player start | The kit gives a start position for each piece: a point outside the blocked area after the game grows it by the actor half width. The game places the player there. On a `center` piece, the start is the centre of the nearest free cell, with ties broken in a fixed order. Implemented: the start is the point nearest the cell centre, away from the opaque side, outside the grown blocked area. A free cell is a cell with no edge and no object. Distance is straight-line, ties go clockwise from north. |
| Draw order | `layers.rs` names two edge positions: ground at -300 (above the background tiles, below the grid debug) and canopy at +150 (above the world band, below the debug drawings). Each game uses one. Long Way uses the ground (changed on 2026-11-15 from the canopy): the player, ghosts, trees and other objects draw over the edge. |
| Art file | `games/long-way/assets/art/features/`, drawn by the user (final art). Long Way draws `map_edge_ground.aseprite`. `map_edge_canopy.aseprite` (for a canopy layer) and `map_edge.aseprite` (the first version) are kept. Each is 64×64, 9 frames, one tag per piece with the `EdgePiece` names, a hidden `debug` layer. |
| Kit and game | The kit owns the parser change, the piece rule, the quarter rules, the errors, the sprite list and placement, the blocked rectangles and the start position. The game chooses its `EdgeMode` and edge layer, maps pieces to frames, and spawns the sprites. |
| Smooth corners | Decided after the spike (step 4, `adr-x49`): the kit supports both. Each game passes `EdgeMode::Basic` or `EdgeMode::Smooth`, set once per game, not per map. Long Way uses `Smooth`. A half-size corner uses the same step size as a full-size corner, so it has half as many steps. |

## Steps

Every step ends with `./dev-scripts/rust.sh lint` and the tests of each crate it changed. Ask before each commit.

### 0. Long Way: redraw maps 1 to 3 in the new layout

So the maps can be used for manual testing in every later step.

- Redraw the grids: `.` for edge cells, blanks for free space. Keep `".": empty` in the legend for now. Today's parser reads `.` and blanks the same, so the maps load and play as before, without edges. Step 5 changes the legend to `".": edge`, and the edges then appear.
- Draw every footprint cell inside the map with a blank.
- Add an exit area to `3.txt` that leads to `1.txt` with `transition: scene`, and an entry area of the same length in `1.txt` (map format rules 16 to 22). Draw both areas with blanks in the outer ring. Propose the positions to the user.
- Follow the rules of the brief, so step 5 does not fail on load:
  - No cell with exactly two opposite free sides, and no cell with four free sides.
  - No `.` in an exit or entry area.
  - Put one `@` on a ring cell that becomes `center` in one map, to check the nearest free cell start in step 6. Its nearest free cell must be inside the map and reachable.
- Propose an edge ring and edge shapes for each map. The user edits or approves them. Cover each piece, each quarter rule, and an edge start, across the three maps.
- Points to show the user:
  - Map 1 has `@` at col=4, row=0, a `south` piece. Map 2 has `@` at col=9, row=0, a `center` piece in the corner of the ring: its nearest free cell is col=8, row=1. Map 3 has `@` at col=0, row=8, a `west` piece.
  - Objects on the ring now stand on edge cells and are blocked on one half. Check that the player can still touch:
    - the exit markers: map 1 `m` at col=3, row=9 (a `north` piece) and map 2 `m` at col=9, row=5 (an `east` piece);
    - the pickups: map 2 `1` at col=8, row=9.
    Move an object inward when it cannot be reached.
  - Enemy spawns on the ring (map 2 `!` at col=0, row=9, a `center` corner, and col=9, row=3, an `east` piece) spawn inside an edge blocker. The user chose to keep them there and test them in step 6.
  - Keep at least one blocking feature on the ring (for example map 1, `g` at col=0, row=3) for the checks in step 6.
- Check: `./dev-scripts/rust.sh test long-way`, and one scripted run per map with `--debug`. Each map loads and plays as before. The new exit area in map 3 sends the player to the entry in map 1.

### 1. Kit: parse edge cells

- Allow `".": edge` in the legend. `".": empty` keeps today's behaviour. Any other value is an error that names the line.
- `Map` gets the edge cells, numbered from the bottom left like `MapObject.cell`. An object cell on the outer ring is an edge cell, unless it is in an entry area. An object cell inside the map is free.
- Error: `.` inside an exit or entry area, naming the line of the area.
- Update the module comments that say `.` is always empty.
- Tests in `map.rs` and `map_areas.rs`:
  - `edge` and `empty` give different cell sets from the same grid. Blank cells are free.
  - An object on each of the four sides of the outer ring, and in a corner of the ring, is an edge cell. Use more than one kind of object (a feature, a pickup, `@`).
  - An object inside the map is free. `@` in an entry area on the ring is free.
  - A `.` footprint cell is an edge and a blank one is free.
  - Both origins give the same cells.
  - Each error message names its line.
  - Make the existing "always empty" test stricter instead of adding a copy.
- No change to Long Way or R-Town yet: their maps say `".": empty`.

### 2. Kit: choose the pieces

- New module `crates/game-kit/src/map_edge.rs`: `EdgePiece` (the 9 pieces) and a plain function that turns the edge cells of a `Map` into one piece per cell, with steps 1 and 2 from the brief. Cells outside the map count as edge.
- Error for a cell with exactly two opposite free sides, or four free sides. It names the grid line of the cell, so the parser must keep the line of each row. The check runs inside `parse` when the legend says `edge`, so a bad map fails on load like other format errors.
- Tests:
  - Every row of the step 1 table, including the four cases with three free sides, and both origins.
  - Each step 2 case from the brief: between two corners, next to a straight piece that faces another way, at the end of a diagonal, next to a corner on each axis.
  - Step 2 does not spread. A straight piece between two `center` cells stays straight.
  - The whole example layout from the brief gives the expected grid.
  - An object cell on the ring gets a piece and counts as edge for its neighbours. An entry area on the ring counts as free for its neighbours.
  - Both error cases name their line.
- Check the maps: load maps 1 to 3 with `".": edge` in a scratch check (not a committed test, `adr-w7`). Fix any map that fails before step 3.

### 3. Kit: blocked rectangles and start position for each piece

- A function from (piece, cell rectangle, step size) to the rectangles that block. Straight pieces give one half-cell rectangle. `center` gives the whole cell. A corner gives a stair of rectangles under its diagonal. The game grows them by its actor half width, as for other blockers.
- A function from (piece, cell rectangle, step size, actor half width) to the start position: a point outside the grown blocked area, inside the cell. For `center`, the start is the centre of the nearest free cell (by grid distance, ties broken in a fixed order, for example north, east, south, west). The function needs the free cells of the map for this case. A map with no free cell is a load error.
- Neighbouring edge cells merge in `outline::build` as usual. Check that the stairs of two neighbouring corners join without a gap that the feet can walk through (`outline::FILLER_HALF`, `adr-x29`).
- Tests:
  - Each piece at two cell sizes and two origins, and a corner at two step sizes.
  - The union of a corner's rectangles is exactly half the cell area when the step count is even.
  - A straight piece's rectangle is on the side named by the piece.
  - For each piece, the start position is outside every grown rectangle and inside the cell. Test two half widths. Test that `resolve` does not let a move from the start position into the blocked half.
  - A `center` start goes to the nearest free cell. Test a case where the nearest cell is one cell away, a case where it is two cells away, and a tie, at two origins.

### 4. Spike: basic edges or smooth edges

A stand-alone step. Its result is a decision by the user, not merged code.

- **Question:** can smooth corners be built cleanly in `game-kit`, without spilling a lot of complexity into the game code or the art?
- **Possibilities:**
  - **Basic:** steps 1 and 2 only (`map_edge_example.aseprite`).
  - **Smooth:** steps 1 and 2, then the quarter rules (cut, fill, and a fill skipped next to a cut) on the whole map, drawn with half-size pieces (`map_edge_example_smooth.aseprite`, the brief's "Quarter rules"). The rules are decided on the basic rendering, so they do not spread.
- **Work:**
  - Extend the output of step 2 so one cell can carry four quarter pieces.
  - Extend step 3 to give the half-size stairs: the same step size, half as many steps.
  - Extend the start position to a cell with quarter pieces.
  - Make a kit test that reproduces both example files cell by cell.
  - Try a game-side draw: one sprite per piece or quarter, with a scale of 0.5 for quarters and the same frames.
  - Work in a worktree, so the result can be dropped.
- **Criteria to report to the user:**
  - The game's job stays "map a piece to a frame, place it at a rectangle and a scale, and set the z-order". Count what else the game would need.
  - The art needs no new frames.
  - Kit cost: lines of rule code and number of tests. The quarter rules need the open and closed state of each quarter side, including the half-filled quarters of the diagonal pieces. The draft script worked from pixels. The kit must work from the piece shapes, without the art.
  - Collision: half-size stairs merge with their neighbours without gaps.
- **Result (done):** both, chosen by the game (`adr-x49`). The quarter rules reproduce the brief's 13 changes exactly. The kit gives the game a list of sprites (`edge_sprites`, placed with `EdgeSprite::place`): a frame at a position and a scale of 1 or 0.5, and for a kept quarter of a cut cell, one quarter of the frame (`Sprite.rect`). No new frames. A cut cell does not draw quarters that are clear in the piece's shape, so art that reaches past the half does not show beside a cut. It can still show beside a fill. The start position needs no change for smooth mode (see `edge_start_feet`). The kit code is merged with its tests. The spike's game draw was not merged. Step 5 writes it again with the kit API.

### 5. Long Way: draw and block the edges

- Use the art `assets/art/features/map_edge_ground.aseprite` (drawn by the user). Map each `EdgePiece` to the tag of the same name (`EdgePiece::name`). Draw only the visible layers (the `debug` layer is hidden).
- Draw order: add the ground (-300) and canopy (+150) edge positions to `layers.rs`, with `const` asserts for their order. Long Way uses the ground position. Do not use bare z numbers (`adr-x9`, `adr-x14`).
- A Long Way constant `EDGE_MODE = EdgeMode::Smooth`. `edge_quarters(EDGE_MODE, ...)`, then spawn one sprite for each item of `edge_sprites`, placed with `EdgeSprite::place` (centre, scale, `Sprite.rect`), tagged with the map id so it is removed with the map and on F1 reload. Nearest-neighbour sampling as for the other art. Half-size pieces draw just above the full-size pieces.
- Add `edge_blocked_rects` to `CollisionWorld`, grown by `PLAYER_HALF_WIDTH`. Start with a step of 16 px and tune it by playing.
- Place the player with `edge_start_feet`: the first start (`player.rs`) and an arrival at `player_start` through an exit object. The second runs in the kit (`map_transition.rs`), so `MapTransitionPlugin` takes `edge_fit: Option<EdgeFit>` (step and half width). Long Way passes `EDGE_FIT`.
- Long Way tests only for what is not in the kit: a wiring test that `spawn_edges` spawns the kit sprites (tag = `EdgePiece::name`, placement, ground z, half-size pieces above) and the blocked areas grown by `PLAYER_HALF_WIDTH`. The tags are the piece names, so there is no separate table to test.
- Change the legend of maps 1 to 3 to `".": edge` (step 0 drew the grids). Run each map with `./dev-scripts/rust.sh script long-way "wait:0.5;screenshot:map<n>;quit" <n>.txt --debug` and check the pieces against the rules by eye.

### 6. Check in the game

- Scripted runs with `--debug` and screenshots (`./dev-scripts/shot.sh` for close-ups):
  - The player starts on an edge cell (map 3, col=0, row=8, a `west` piece) at the kit's start position, outside the blocked area. The player can move right and cannot move left. Do the same on a `south` piece (map 1, col=4, row=0). In map 2 (`@` at col=9, row=0, a `center` piece), the player starts at the centre of col=8, row=1.
  - Walk into each straight piece and each corner. Move diagonally along a stair, also a half-size one at a cut and a fill.
  - Close-ups of each fill in map 3: how much of the `north` and other frames shows past the half beside a fill. Show the user.
  - A ghost is stopped by the edge and paths around it.
  - Ghosts that spawn on the ring (user decision, keep and test): map 2 `!` at col=0, row=9 spawns in a fully blocked `center` corner, and map 2 `!` at col=9, row=3 in the blocked half of an `east` piece. Check that each one walks out, finds a path, and chases the player. If a ghost is stuck, write a failing kit test first, then fix, and show the user.
  - The edge draws below the player, the ghosts and the trees (ground position). With F3, the grid draws over the edge.
  - A blocking feature on the edge (map 1, `g` at col=0, row=3): the edge piece under it blocks, and the trunk blocks as before. Show the blocked areas (F3) to the user with a screenshot.
  - Objects to touch on the ring (user decision, keep and test): the player can touch map 1 `m` at col=3, row=9 (`north` piece), map 2 `m` at col=9, row=5 (`east` piece) and the pickup map 2 `1` at col=8, row=9 (`north` piece). If one cannot be touched, show the user before changing the map or the touch rule.
  - The exit area in map 3 sends the player to the entry in map 1, and both areas are gaps in the edge.
- Say which parts were not seen on screen (motion feel, timing).
- `./dev-scripts/rust.sh lint`, `test game-kit`, `test long-way`, `build r-town`, `web`.
- Delete the screenshots. Run `./dev-scripts/rust.sh size`.

### 7. Final step: migrate and delete

- `dev-docs/extensions/map-files.md`: rules 3, 8, 12, 20 and 21 (`.`, blanks, footprints drawn with blanks, the new area error, objects on the ring), a new rule for edges (pieces, errors, objects and `@` on the ring, the start position), the new tests in "Tests", the R-Town note.
- `dev-docs/adr/EXTENSIONS-MAPS.md`: new `adr-x` entries (next free number): `.` can be an edge (a fixed legend value), the kit owns the edge rules and the start position, objects on the ring stand on edge cells, footprints follow the file. `adr-x49` (kit owns the rules, the game chooses `EdgeMode`, the spike result) is written. `dev-docs/adr/EXTENSIONS-SCREEN.md` or the layers ADR: one edge layer per game, at the ground or the canopy position.
- `games/long-way/docs/adr.md`: the Long Way choices (ground layer, smooth mode, art file, step size), next free `adr-lw` number.
- `games/long-way/docs/maps.md` (the new exit area and entry), `art-pipeline.md` (the edge art file), `collision.md` (edge blockers), `source-layout.md` (new code).
- `dev-docs/bevy/notes.md`, only if a new verified Bevy pattern came up.
- `CLAUDE.md`: the kit list in "Games and workspace rules" (map edges).
- Move any lasting content of the brief (the rule tables, the examples) into `map-files.md`, then delete the brief and this plan. The drawn examples stay in `input/`.
