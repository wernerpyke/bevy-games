# Plan: rock clusters (then forests) with map files

Status: planning. No code is written. Four small decisions are open (section 8).
Updated: 2026-11-01. Related: roadmap item 1, `adr-lw1` (blocked band), `adr-lw4` (`allow_sliding`), `adr-lw7` (real-time RPG), `adr-lw8` (maps and rocks), roadmap item 3 (extract collision).

## 1. Goal

1. A human describes a cluster of rocks in a markdown file. The game builds the rocks and their collision from it.
2. Rocks are solid. Guy can only walk around the outside of a cluster, and he slides around its edges as he does for a tree trunk today.
3. Trees and forests later use the same map format and the same outline collision.

Long Way is a real-time top-down RPG (`adr-lw7`). Movement and sliding use frame time. NPCs and pathfinding are expected later.

## 2. Decisions already made

| Topic                   | Decision                                                                                                                                    |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Game type               | Real-time top-down RPG, not turn-based, not grid-locked (`adr-lw7`)                                                                         |
| First shape             | Rocks before trees. `Rocks.aseprite` exists: 128x128, two frames, tags `rock_0` and `rock_1`                                                |
| Rock behaviour          | Solid on all sides. No walk-behind part. Walk around the outline only                                                                       |
| Rock size               | 2x2 cells, 128 pixels                                                                                                                       |
| Rock collision          | Rectangle inset 10% on each side of the 128 pixel footprint                                                                                 |
| Vertical margin         | 0 above and below the inset rectangle (decision 1, agreed)                                                                                  |
| Drawn look              | Rocks are drawn slightly overlapping, like shrubs in a top-down RPG. Collision does not follow the overlap, it stays on the inset footprint |
| Layout                  | Paint cells as text in a markdown file with frontmatter                                                                                     |
| Map cell (0, 0)         | Fixed world position `MAP_ORIGIN` = (-1024, -576), the F2 grid uses it too, so cells do not move when the window is resized                 |
| Placement, slide range  | Not decided. Experiment first (sections 5 and 6)                                                                                            |
| Loader and parser       | `include_str!` and an own parser for now. A more robust approach is on the roadmap (item 6)                                                 |
| Scope of this iteration | Includes the primitive map-making (parse file, place objects, collision), not only collision                                                |

## 3. What the code does today

From `games/long-way/src/collision.rs`, `trees.rs`, `debug.rs`:

- Each `Trunk` has a blocked rectangle for Guy's **feet**. Guy's body width (20 each side) is already added, so the feet are one point.
- `resolve` tests every trunk, one axis at a time. `move_feet` slides Guy along one rectangle face only, with a 0.2 second delay and a 35% range rule.
- Draw order uses the feet line (`depth.rs`), so overlapping sprites already sort.
- The F2 grid numbers cells from the bottom left, 64 pixels each. Map files use the same cell numbers.

## 4. Rock geometry

- Footprint: 2x2 cells = 128 x 128. Inset 10% each side: 102.4 x 102.4.
- Blocked area for the feet: the inset rectangle grown sideways by Guy's half width (20). Result 142.4 wide.
- Two rocks whose blocked areas touch or overlap join into one cluster. Rocks 3 or more cells apart leave a gap of about 50 pixels that Guy can walk through, so they are separate clusters.
- Everything inside a cluster is unreachable, so interior rocks need no collision test.

The vertical margin is 0: the blocked area ends at the inset rectangle above and below. Guy standing below a rock has his upper body over the lower part of the sprite, as with trees.

### Drawn overlap (facts from the art)

- `rock_0.png` and `rock_1.png` are 32x32 pixel art scaled 8x to 256x256. Checked: every 8x8 block is one colour.
- The opaque part of the rock is 232 x 208 of 256 (x 8 to 239, y 16 to 223): 90.6% of the width and 81% of the height.
- Our `Rocks.aseprite` is 128x128, so one art pixel is 4 screen pixels. The rock is about 116 wide. At a pitch of 2 cells (128) two neighbours do **not** touch: 12 pixels of gap.
- To overlap slightly at a pitch of 2 cells, draw the rock larger than its footprint. 160 pixels is exactly 5x the art, so pixels stay even. The rock is then about 145 wide and neighbours overlap by about 17 pixels (about 12%). 192 pixels (6x) overlaps by about 46 pixels.
- `Rocks.aseprite` is therefore kept at the native 32x32 and scaled in the engine by `draw / 32`, which must be a whole number (4 = 128, 5 = 160, 6 = 192). Pixels stay even. Stretching a 128 pixel file to 160 would not (4x to 5x).
- Drawing larger than the footprint does not change collision. The inset footprints are 25.6 apart at a pitch of 2 cells, and Guy's 20 pixel margin each side closes that gap, so the blocked areas join and a cluster has no gaps.

## 5. The map file (human interface)

File location: `games/long-way/assets/maps/<name>.md`. The body is ordinary markdown, so a human can write notes, headings and sketches around the maps. Only the frontmatter and the fenced `cells` blocks are read.

````markdown
---
map: rock-cluster-test
cell: 64
origin: bottom-left
legend:
  ".": empty
  "0": rock_0
  "1": rock_1
---

# Rock cluster test

Free text. The game ignores this. Use it to explain the layout.

```cells at=10,6
. . 1 1 . .
. 0 0 1 1 .
. 0 0 . . .
. . 0 0 . .
```
````

Rules:

1. **Frontmatter.** `map` is a name for log messages. `cell` is the cell size in pixels (64). `origin: bottom-left` says cell (0, 0) is the bottom left cell, as in the F2 grid. `legend` maps one character to an object kind. `.` is always empty. Kinds are defined in code (`rock_0`, `rock_1` now, trees later).
2. **Cell block.** A fenced block that starts with `cells at=x,y`. `x,y` is the cell of the **bottom left character**. Rows are written top to bottom, so the first row has the highest y. Characters are separated by spaces. A map can have several blocks.
3. **Placement.** A character places one object whose **bottom left corner** is that cell. A rock covers 2x2 cells, so it extends one cell right and one cell up. Neighbouring characters overlap by one cell, which makes a pile. A solid 4 x 3 painted area gives a cluster of 5 x 4 cells.
4. **Errors.** An unknown character, a ragged row or a missing `at=` stops loading and names the file and line. A silent skip would hide layout mistakes.
5. **Check.** The F2 grid and the `screenshot:` script command let Claude and the human compare the file with the screen.

The example above gives 10 rocks. Blocked outline: one cluster, about 6 x 5 cells.

### Format choices to confirm

- **Parser.** A small own parser for the flat frontmatter (`key: value`, plus the `legend` list) needs no dependency. `serde_yaml` is archived and unmaintained, so a YAML crate would be a poor base. Recommended: own parser, with the supported subset written in the file header and covered by tests.
- **Loading.** Recommended first: `include_str!` of the map file. Edit the file, rebuild (about 8 seconds), no loader code, safe on wasm. Later: a Bevy asset loader with hot reload.

### Placement (decided: V1)

Chosen on 2026-11-01 after comparing three variants on screen: **one character per cell (`pitch: 1`), rocks drawn at 128 px (`draw: 128`), no jitter**. A dense pile with 50% overlap and fine control of the cluster shape. The variants with a 2 cell pitch (regular boulders, with and without 8 px jitter) were dropped. `pitch`, `draw` and `jitter` stay as optional frontmatter keys with these defaults, because shrubs or other kinds may want a different look later. Size rule: `n` characters in a row cover `pitch * (n - 1) + 2` cells.

### Reusable clusters (step 9)

A cluster can be defined once and placed several times, so a good shrub patch is drawn once and reused across the map. Proposed format, added after the basic format works:

````markdown
```cells name=shrub-patch
. 0 0 .
0 1 1 0
. 0 0 .
```

```place shrub-patch
at=20,6
at=40,12
at=5,30
```
````

Rules:

1. A `cells` block with `name=` and no `at=` is a **template**. It places nothing by itself. Its rows are read exactly like a normal block, with the bottom left character as the anchor.
2. A `place <name>` block has one `at=x,y` per line. Each line places a copy of the template with its bottom left character at that cell.
3. A template must be defined before it is placed. Names are unique. A template cannot contain `place` lines (no nesting in the first version). Errors name the file and line.
4. A normal `cells at=x,y` block still works and can sit beside templates in the same file.
5. Each copy is the same shape, but `jitter` uses the world cell, so copies look slightly different if jitter is on. Mirroring or rotating a copy is not planned.
6. Collision needs nothing new: copies are ordinary objects, so overlapping copies merge in the outline.

### Extending to trees

Each object kind has art, a footprint and a collision rule, defined once in code:

| Kind | Art | Footprint | Collision |
|---|---|---|---|
| `rock_0`, `rock_1` | `Rocks.aseprite` tag | 2x2 cells | solid, inset 10% |
| `green_tree`, `red_tree` (later) | tree art | 2x2 cells | trunk band (`adr-lw1`) |

The map file only names kinds. Adding a kind never changes the file format.

## 6. Collision approaches (research summary)

All blocked areas are axis-aligned rectangles, so the union is a rectilinear shape (horizontal and vertical edges only). The axis based slide rule keeps working on edge segments. Merging also removes seams, where a diagonal move catches on the inner edge between two touching rectangles.

| | A neighbour aware | B boundary polygon | C computed union (+ D cull) | E occupancy grid | F distance field | G physics crate |
|---|---|---|---|---|---|---|
| What | Keep rectangles, slide along a run of touching ones | Hand drawn forest outline | Merge rectangles at startup, collide with the outline | Rasterise blocked areas to a fine grid | Distance field and normal | `avian2d` or `rapier` |
| Authoring effort | none | high | none (cells already painted) | none | none | medium |
| Fixes seams | no | yes | yes | yes | yes | yes |
| Slide along long edges | partly | yes | yes | yes | different rule | different rule |
| Runtime cost | O(objects) | O(edges) | O(edges), less with D | O(1) | O(1) | engine |
| New dependency | no | maybe | no for rectangles | no | no | yes |
| Works on wasm | yes | likely | yes | yes | yes | to check |
| Unit testable without Bevy | yes | yes | yes | yes | yes | harder |
| Feeds NPC navigation later | no | yes | yes | yes | no | engine |

- **D (cluster, then skip interior objects)** is the developer's second idea. It only saves work. It is cheap once C has the union, so it is a step inside C.
- **B** matters for non-rectangular shapes. The painted map already authors the shape, so B is not needed for rocks.
- **E** is a candidate later for NPC pathfinding, not for Guy's collision.
- **F and G** do not fit: the slide is a game rule (delay, nearest edge, range limit), and the 50% overlap rules are not normal box collision. Roadmap item 4 decides the physics question.
- The outline already includes Guy's width. The same grown shapes are what a navigation mesh or visibility graph for NPCs needs ("configuration space"), so the data can be reused.

**Recommendation: C with D**, using own rectilinear union code (coordinate compression: sort all x and y values, mark filled cells, trace the boundary). It is exact, about 60 lines, testable and has no dependency. Bring in `i_overlay` (9.1.0, union and offset, `f32` and `f64`) only for non-rectangular shapes. Its repository is tagged `wasm` and `no-std`, but a wasm build is not tested.

```mermaid
flowchart LR
    M[Map file .md] --> P[Parser: frontmatter and cells]
    P --> O[Objects: kind and cell]
    O --> S[Spawn sprites from Rocks.aseprite]
    O --> R[Blocked rectangle per object]
    R --> U[Union per touching group]
    U --> L[Outline edges]
    U --> I[Mark interior objects]
    L --> RES[resolve: feet vs outline]
    L --> SL[slide: target on connected edge]
    I --> RES
```

### What the move to a real-time RPG changes

The recommendation for **Guy's collision stays C with D**. The feet-point rule, the outline and the slide rule do not depend on turns. What changes is the design around it:

1. **Many moving entities.** NPCs and enemies will test the same outline. Store it as one static resource with a broad phase (a spatial hash of edges) once the edge count grows. Not needed for the first iteration.
2. **Dynamic obstacles.** Doors, pushable objects and destroyed rocks change the outline. Rebuild the affected cluster only, or keep dynamic obstacles in a separate list. Static clusters stay precomputed.
3. **Navigation.** NPCs need pathfinding. The grown outline is the configuration space a navigation mesh or visibility graph needs, so it should be a reusable data structure, not buried in `resolve`. Occupancy grid (E) or a nav mesh then becomes a separate system fed by the same map.
4. **Larger worlds.** One map file per area, and the union per chunk, not one global outline.
5. **Physics crate.** The question in roadmap item 4 weighs more (NPC against NPC, projectiles, knockback). It does not replace the feet-point rule for terrain.
6. **Time.** Everything already uses frame time, so no change.

### Real-time notes

- Steps are small per frame (300 units per second, about 5 per frame), so tunnelling is not a risk. `resolve` still stops a move at the first edge it crosses.
- NPCs will use `move_feet` as well. Testing every outline edge per entity is fine for dozens of entities. A spatial hash is the later fix if it is not (static objects are inserted once).
- The slide timer already uses `dt`, so no change is needed for real-time play.

### Slide design to settle

- **Range rule.** 35% of a long cluster edge is too far. Options: a cap of one cell (64 px) or 1.5 cells, or the distance to the nearest outline corner with the cap.
- **Give up distance.** Never slide past a limit, so Guy is not carried round a corner onto the next face.
- **Concave corners.** At an inside corner, slide away from the corner, never into it.
- **Push direction.** The assist stores signs, not floats (`adr-lw5`). A merged face has one direction, so this stays.

## 7. Steps

1. **Map parser (done 2026-11-01).** Plain functions with unit tests: frontmatter, legend, cell blocks, errors with file and line. A first map file `assets/maps/rocks-test.md`.
2. **Spawn rocks from the map (done 2026-11-01).** Sprites from `Rocks.aseprite` (variant by tag), `DepthSort`, placed from cell numbers, with the `pitch`, `draw` and `jitter` knobs. Compared three variants on screen next to the F2 grid; the developer chose V1.
3. **Failing tests for the outline (done 2026-11-01).** Two touching rectangles, an L shape, a notch, a gap just wide enough for Guy, a gap too narrow.
4. **Union and outline (done 2026-11-01)** in `outline.rs` (plain functions, only `Rect` from Bevy), by coordinate compression. 18 tests. The grid is built over all rectangles at once, so very large maps need chunking (roadmap item 6).
5. **`resolve` against the outline.** Trunk collision keeps working through the same tests.
6. **Slide target on the connected outline edge**, with the range and give up rules.
7. **Interior cull (D)**, with a test that culled objects change no result.
8. **Collision overlay** (started: `rocks.rs` draws the rock outline in yellow with ticks to the blocked side whenever F2 is on, 2026-11-01. Still to do: interior rocks in another colour, and the trunk overlay following F2.) draws the outline, interior objects in another colour. Decided: it turns on and off together with the F2 grid, so F1 is no longer a separate switch (the `ShowCollision` resource in `collision.rs` follows `DebugSettings.show_grid`).

9. **Reusable clusters.** Define a cluster once and place it several times on the map (section 5, "Reusable clusters"). Parser: a `cells name=<name>` block defines a template, a `place <name>` block lists `at=x,y` positions. Tests first: one template placed twice, overlapping placements, an unknown template name, a template used before it is defined, a name defined twice, nesting (not allowed in the first version). The outline then merges overlapping placements with no extra code. Screenshot a map that places one cluster three times.

10. **Check in the real game** with `script` and `screenshot:`: push into a long face, slide past a cluster corner, push into a notch, walk through a narrow gap, depth order of overlapping rocks. Report what was not seen on screen.
11. `lint`, `test`, `web`.
12. **Record** decisions in `dev-docs/adr/LONG-WAY.md` (collision and map format, `adr-lw9` onward). Add the outline and map format to `BEVY-ADDITIONS.md`, new Bevy patterns to `bevy-notes.md`, kind tags to `art-tags.md`. Update roadmap item 3 (the general versus Long Way split in `collision.rs`).
13. **Migrate lasting content** from this plan into long-lived docs (a user guide for map files in `docs/`, decisions in `dev-docs/adr/`), then delete this file. Ask the developer before the commit.

## 8. Open decisions

1. **Vertical margin:** decided, 0. Rocks are drawn slightly overlapping (section 4).
2. **Placement meaning:** decided, V1 (section 5).
3. **Slide range** on long edges: experiment. Try a 64 px cap, a 96 px cap and the nearest corner with a cap, using scripted runs and the slide log lines. Keep the three behind one constant or function while testing.
4. **Loading and parser:** decided for now: `include_str!` and an own parser. A more robust approach is roadmap item 6.

## 9. Sources

- [Greedy merging of tile maps into collision rectangles (Feronato)](https://emanueleferonato.com/?p=15637) and [gamedev.net copy](https://gamedev.net/news/2165-reducing-tiled-maps-to-optimized-collision-rectangles-with-greedy-merging/): grid to few rectangles, a one time pass, removes seams.
- [FlatRedBall merged collision](https://docs.flatredball.com/flatredball/tiled-plugin/glue-gluevault-component-pages-tile-graphics-plugin-tileshapecollection/addmergedcollisionfromtileswithproperty): merges adjacent rectangles by row or column.
- [Unity CompositeCollider2D](https://docs.unity3d.com/ja/560/ScriptReference/CompositeCollider2D.html): merges box and polygon colliders into one outline.
- [Defold forum: top-down static colliders](https://forum.defold.com/t/what-is-the-best-way-to-set-up-collisions-in-static-sprite-based-2d-games-with-a-top-down-view/73757): concave shapes need care.
- [Construct forum: many trees](https://construct.net/en/forum/construct-2/general-discussion-17/lighter-collision-element-98453): collider on the base only, plus spatial partitioning.
- [i_overlay (docs.rs source)](https://docs.rs/crate/i_overlay/latest/source) and [repository](https://github.com/iShape-Rust/iOverlay): union, difference, offset, float and integer points.
- [Adrien Bon, bevy_ecs_tiled physics guide](https://adrien-bon.github.io/bevy_ecs_tiled/guides/physics.html): colliders from Tiled object layers. A later option for maps.
- [Obstacle inflation and Minkowski sum, MIT 6.141 CSpace](https://courses.csail.mit.edu/6.141/spring2013/pub/labs/lab6/docs/javadoc/GlobalNavigation/CSpace.html): grow obstacles by the agent radius so the agent is a point.
- [Godot, using CharacterBody2D](https://docs.godotengine.org/ja/4.5/_sources/tutorials/physics/using_character_body_2d.rst.txt): `move_and_slide` collide and slide.
- [Spatial hash broad phase (Bonk docs)](https://gitea.moonside.games/MoonsideGames/bonk-docs/src/branch/main/content/broad_phase/spatial_hash.md): static objects inserted once.

Not verified: `i_overlay` building for `wasm32-unknown-unknown`, and that `serde_yaml` is archived (from memory, check before relying on it).
