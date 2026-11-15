# PLAN: map edges for Long Way, with the rules in the kit

Source: `dev-docs/BRIEF-map-edges.md` (the brief). The brief holds the rules and the drawn examples. This plan holds the steps. Where the two differ, the brief wins. Fix the plan.

## Status (2026-11-15): resume here

Not started. Next: step 0, then the questions in "Before step 1".

## Goal

In a map file, `.` becomes a map edge and a blank becomes free space. Each edge cell is drawn with a piece of `map_edge.aseprite` and blocks the opaque half of that piece. `game-kit` owns the rules. Long Way owns the art and the draw order.

## What is already in place

- **Parser:** `crates/game-kit/src/map.rs`. `.` is reserved and must be written `".": empty` (the check at the legend, about line 693). A blank or `.` is an empty cell (about line 573). `Map` has objects only, so it does not know which cells are `.`.
- **Areas:** `crates/game-kit/src/map_areas.rs` (exits, entries, `PLAYER_START`). Rule 21 allows `@` in an entry area.
- **Collision:** rectangles only (`adr-x4`, `adr-x10`). The game grows each blocked area sideways by the actor half width (Long Way: `PLAYER_HALF_WIDTH` = 14, `games/long-way/src/collision.rs`). Ghosts path with `NavGrid::build(world.edges(), ...)` (`crates/game-kit/src/nav.rs`), so edge blockers in `CollisionWorld` also block ghosts.
- **Map border:** the player's feet stay in `walkable_areas` (`games/long-way/src/player.rs`). This does not change.
- **Draw order:** `crates/game-kit/src/layers.rs` (`BACKGROUND_Z` -500, `BACKGROUND_TILES_Z` -490, world band ±100, `GRID_DEBUG_Z` -200).
- **Art:** `games/long-way/input/map_edge.aseprite`: 64×64, 9 frames, one tag per piece (`north` ... `center`). The frames have debug letters on them. Drawn examples: `input/map_edge_example.aseprite` and `input/map_edge_example_smooth.aseprite` (`input/` is git-ignored).
- **Maps:** Long Way `assets/maps/1.txt` to `3.txt` use `".": empty`. R-Town `assets/maps/test.txt` uses it too and does not change.

## Out of scope

- R-Town. It keeps `".": empty`.
- A tile layer (`rm3`, `ROADMAP.md`).
- Inside corners, except as part of the smooth corner spike (step 4).

## Before step 1: questions for the user

Each has a recommendation. Record the answers in "Decisions" and in the ADRs in step 7.

1. **Legend value.** Recommended: `".": edge`, a value the kit knows, like `empty`. `.` may only be `empty` or `edge`. The other choice is a free kind name that the game passes to the kit. That adds a game-defined name to a rule the kit owns.
2. **`.` inside an exit or entry area.** Today rule 20 only advises blanks. A `.` in an area would now block the exit or the entry. Recommended: a load error that names the line of the area.
3. **Art file name.** The frames have debug letters on them. Recommended: copy as `assets/art/backgrounds/map_edge_placeholder.aseprite`, and drop the suffix when the final art arrives.
4. **Long Way draw order.** Each game has one edge layer: ground (below the player, shrubbery) or canopy (above the player, tree canopy). Which one does Long Way use?
5. **Footprints on an edge.** The anchor cell of a feature is free space (brief). But the other cells of its footprint are drawn with `.` or blanks (map format rule 12), and the kit does not know footprints (rule 11, `adr-x13`). Example: map 1, `g` at col=0, row=3 has a 2×2 footprint that reaches col=0, row=4 on the outer ring. Recommended: the file is the truth. A footprint cell drawn `.` is an edge under or over the feature, and a blank is free. The kit stays without footprints. The other choice: the game passes the footprints to the kit, and footprint cells are always free.

## Decisions (from the brief)

| Topic          | Decision                                                                                                                                                                                                                                 |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Cells          | `.` is an edge. A blank (`  ` or ` `) is free space.                                                                                                                                                                                     |
| Piece          | The brief's rule step 1 (free sides) and rule step 2 (a straight piece next to a different piece along its wall becomes `center`). See the brief, "Choosing the piece".                                                                  |
| Not supported  | A cell with free space on opposite sides, or on four sides: a load error that names the line.                                                                                                                                            |
| Blocking       | The opaque half of the piece. `center` blocks the whole cell. Corners block with stair-step rectangles. The step size is tuned by playing.                                                                                               |
| Objects        | A cell with an object is free space. The object blocks as it does today.                                                                                                                                                                 |
| `@`            | On the outer ring of the map, `@` is an ordinary edge cell: it is drawn, it blocks, and its neighbours treat it as edge. The player starts at the centre of that cell. In an entry area, or away from the outer ring, `@` is free space. |
| Kit and game   | The kit owns the parser change, the piece rule, the errors and the blocked rectangles. The game maps pieces to frames and chooses the draw order.                                                                                        |
| Smooth corners | A spike (step 4) decides between basic edges and smooth edges.                                                                                                                                                                           |

## Steps

Every step ends with `./dev-scripts/rust.sh lint` and the tests of each crate it changed. Ask before each commit.

### 0. Long Way: redraw maps 1 to 3 in the new layout

So the maps can be used for manual testing in every later step.

- Redraw the grids: `.` for edge cells, blanks for free space. Keep `".": empty` in the legend for now. Today's parser reads `.` and blanks the same, so the maps load and play as before, without edges. Step 5 changes the legend to `".": edge`, and the edges then appear.
- Follow the rules of the brief, so step 5 does not fail on load:
  - No cell with free space on opposite sides or on four sides.
  - No `.` in an exit or entry area.
  - `@` on the outer ring is an edge start.
- Propose an edge ring and edge shapes for each map. The user edits or approves them. Cover each piece, each quarter rule, and an edge start, across the three maps.
- Points to show the user:
  - Maps 1 and 2 have `@` in row 0. It becomes an edge start on a `south` piece, so the player cannot move down at the start.
  - Map 1 has the exit marker `m` in row 9. On the ring it is free space, so it leaves a gap in the drawn edge. The map border still keeps the player in.
  - Blocking features on the ring, for example map 1, `g` at col=0, row=3. Its cell is free space, so the edge has a gap where the tree stands. Its footprint cells follow question 5. Keep at least one such case in the maps for the checks in step 6. Two features one cell apart on the ring make the edge cell between them one cell thick (an error from step 2), so avoid that or move a feature.
- Check: `./dev-scripts/rust.sh test long-way`, and one scripted run per map with `--debug`. Each map loads and plays as before.

### 1. Kit: parse edge cells

- Allow `".": edge` in the legend (question 1). `".": empty` keeps today's behaviour. Any other value is an error that names the line.
- `Map` gets the edge cells, numbered from the bottom left like `MapObject.cell`. A cell with an object is not an edge cell. The exception is an `@` on the outer ring and not in an entry area.
- Error: `.` inside an exit or entry area (question 2).
- Update the module comments that say `.` is always empty.
- Tests in `map.rs` and `map_areas.rs`: `edge` and `empty` give different cell sets from the same grid. Blank cells are free. An object cell is free, also for a blocking feature whose anchor is on the outer ring. Footprint cells follow question 5 (with the recommended answer: a `.` footprint cell is an edge, a blank one is free). `@` on each of the four sides of the outer ring is an edge, `@` inside the map is free, and `@` in an entry area on the ring is free. Both origins give the same cells. Each error message names its line. Make the existing "always empty" test stricter instead of adding a copy.
- No change to Long Way or R-Town yet: their maps say `".": empty`.

### 2. Kit: choose the pieces

- New module `crates/game-kit/src/map_edge.rs`: `EdgePiece` (the 9 pieces) and a plain function that turns the edge cells of a `Map` into one piece per cell, with steps 1 and 2 from the brief. Cells outside the map count as edge.
- Error for a cell with free space on opposite sides or on four sides. It names the grid line of the cell, so the parser must keep the line of each row. Decide whether the check runs inside `parse` (then every game gets it) or in the new function. Recommended: inside `parse`, called when the legend says `edge`, so a bad map fails on load like other format errors.
- Tests: every row of the step 1 table, including the four cases with three free sides, and both origins. Each step 2 case from the brief (between two corners, next to a straight piece that faces another way, at the end of a diagonal, next to a corner on each axis). Step 2 does not spread. A straight piece between two `center` cells stays straight. The whole example layout from the brief gives the expected grid. Both error cases name their line.
- Tests for features on the edge: the neighbours of a feature cell on the outer ring choose their pieces as if the cell is free (on each of the four sides of the ring, and in a corner of the ring). Two features one cell apart on the ring make the cell between them an error that names its line.

### 3. Kit: blocked rectangles for each piece

- A function from (piece, cell rectangle, number of steps) to the rectangles that block. Straight pieces give one half-cell rectangle. `center` gives the whole cell. A corner gives a stair of rectangles under its diagonal. The game grows them by its actor half width, as for other blockers.
- Neighbouring edge cells merge in `outline::build` as usual. Check that the stairs of two neighbouring corners join without a gap that the feet can walk through (`outline::FILLER_HALF`, `adr-x29`).
- Tests: each piece at two cell sizes and two origins, and a corner at two step counts. The union of a corner's rectangles is exactly half the cell area when the step count is even. A straight piece's rectangle is on the side named by the piece.

### 4. Spike: basic edges or smooth edges

A stand-alone step. Its result is a decision by the user, not merged code.

- **Question:** can smooth corners be built cleanly in `game-kit`, without spilling a lot of complexity into the game code or the art?
- **Possibilities:**
  - **Basic:** steps 1 and 2 only (`map_edge_example.aseprite`).
  - **Smooth:** steps 1 and 2, then the quarter rules (cut, fill, and a fill skipped next to a cut) on the whole map, drawn with half-size pieces (`map_edge_example_smooth.aseprite`, the brief's "Quarter rules"). The rules are decided on the basic rendering, so they do not spread.
- **Work:**
  - Extend the output of step 2 so one cell can carry four quarter pieces.
  - Extend step 3 to give the half-size stairs.
  - Make a kit test that reproduces both example files cell by cell.
  - Try a game-side draw: one sprite per piece or quarter, with a scale of 0.5 for quarters and the same frames.
  - Work in a worktree, so the result can be dropped.
- **Criteria to report to the user:**
  - The game's job stays "map a piece to a frame, place it at a rectangle and a scale, and set the z-order". Count what else the game would need.
  - The art needs no new frames.
  - Kit cost: lines of rule code and number of tests. The quarter rules need the open and closed state of each quarter side, including the half-filled quarters of the diagonal pieces. The draft script worked from pixels. The kit must work from the piece shapes, without the art.
  - Collision: half-size stairs merge with their neighbours without gaps.
- **Result:** the user chooses. Record the choice in the brief and in an ADR (step 7). If smooth wins, merge the spike's kit code with its tests, then go on to step 5. If basic wins, drop the worktree.

### 5. Long Way: draw and block the edges

- Copy the art into the game (question 3). Map each `EdgePiece` to the tag of the same name.
- Draw order: each game has one edge layer, ground or canopy (question 4). `layers.rs` names the two positions, and the game uses the one it chose. Do not use bare z numbers (`adr-x9`, `adr-x14`).
- Spawn one sprite per edge cell (or quarter) for each active map, as a child of the map, so it is removed with the map and on F1 reload. Nearest-neighbour sampling as for the other art.
- Add the blocked rectangles to `CollisionWorld`, grown by `PLAYER_HALF_WIDTH`. Use the step size from step 3 (start with 4 steps of 16 px, tune by playing).
- Long Way tests only for what is not in the kit: the piece-to-tag table covers every `EdgePiece` (a test fails on a missing name).
- Change the legend of maps 1 to 3 to `".": edge` (step 0 drew the grids). Run each map with `./dev-scripts/rust.sh script long-way "wait:0.5;screenshot:map<n>;quit" <n>.txt --debug` and check the pieces against the rules by eye.

### 6. Check in the game

- Scripted runs with `--debug` and screenshots (`./dev-scripts/shot.sh` for close-ups):
  - The player starts on an edge cell (map 3, col=0, row=8, a `west` piece). The player can move right and cannot move left. Then check whether the feet start inside the grown blocked area, and whether `resolve` lets the player move out. If the player is stuck, write a failing kit test first, then fix.
  - Walk into each straight piece and each corner. Move diagonally along a stair.
  - A ghost is stopped by the edge and paths around it.
  - A blocking feature on the edge (map 1, `g` at col=0, row=3):
    - The feature and the edge pieces around it are drawn in the right order. The edge layer is below or above the tree, as chosen in question 4.
    - The player and a ghost stay inside the map. The map border holds them, because the tree's cell is free space.
    - The tree's cell is free and the tree only blocks its trunk. So the player may be able to walk between the trunk and the map border, behind the tree. Show this to the user with a screenshot of the blocked areas (F3). Do not change it without a decision.
    - Check the same for one small feature on the ring (a one-cell shrub or a lantern).
  - An exit area gap still sends the player to the next map, and a seamless exit still joins.
- Say which parts were not seen on screen (motion feel, timing).
- `./dev-scripts/rust.sh lint`, `test game-kit`, `test long-way`, `build r-town`, `web`.
- Delete the screenshots. Run `./dev-scripts/rust.sh size`.

### 7. Final step: migrate and delete

- `dev-docs/extensions/map-files.md`: rules 3, 8, 12 and 20 (`.` and blanks), a new rule for edges (pieces, errors, `@` on the ring), the new tests in "Tests", the R-Town note.
- `dev-docs/adr/EXTENSIONS-MAPS.md`: new `adr-x` entries (next free number): `.` can be an edge, and the kit owns the edge rules. Include why the game only maps pieces to frames, and the result of the spike. `dev-docs/adr/EXTENSIONS-SCREEN.md` or the layers ADR: one edge layer per game, at the ground or the canopy position.
- `games/long-way/docs/adr.md`: the Long Way choices (draw order, art file, step size), next free `adr-lw` number.
- `games/long-way/docs/maps.md`, `art-pipeline.md` (the copied art file), `collision.md` (edge blockers), `source-layout.md` (new code).
- `dev-docs/bevy/notes.md`, only if a new verified Bevy pattern came up.
- `CLAUDE.md`: the kit list in "Games and workspace rules" (map edges).
- Move any lasting content of the brief (the rule tables, the examples) into `map-files.md`, then delete the brief and this plan. The drawn examples stay in `input/`.
