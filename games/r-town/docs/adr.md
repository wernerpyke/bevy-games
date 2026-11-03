# R-Town: decisions

## adr-rt1. Pack art enters the pipeline as grouped .aseprite files (2026-11-02)

**Decision.** Aggregate the PNGs of the "Superpowers Medieval Fantasy Asset Pack (part 1)" into one `.aseprite` file per category (characters, monsters, tiles, items, hud), with Aseprite tags per sprite or animation. A Lua script does the import.

**Rationale.** The pack ships PNG and GIF only. Grouped `.aseprite` files let the existing art pipeline (adr-a1) and `bevy_aseprite_ultra` work unchanged, and keep the file count low.

**How to apply.** Raw pack files stay in the gitignored `input/` folder. Only generated or grouped files go into `assets/`. Copy a file only after the developer names it.

## adr-rt2. Personal use inside this repo (2026-11-02)

**Decision.** R-Town is for personal use. Pack files may be committed to this repo. No public build.

**Rationale.** The pack license allows any use but says "do not redistribute".

**How to apply.** Revisit before any public repo or web release.

## adr-rt3. 16x16 cell and hero animated in code (2026-11-02)

**Decision.** The grid cell is 16x16 art pixels. The hero is pack character 9 (14x16 px). The pack has one static pose per character, so the hero is animated in code: left and right flip, a small bob while walking, a squash when idle.

**Rationale.** The pack tileset is a 16 px grid and the hero is 16 px tall, so one cell fits one hero. Drawing four-direction walk cycles costs art time that the first playable does not need. The Aseprite tags `Idle`, `Walk`, `Up`, `Down` that Long Way uses are not available for these sprites.

**How to apply.** Replace the code animation with drawn frames later without changing the hero's size or collision. The camera view height must keep an integer scale at a 16 px cell (open question in `SPEC.md`).

## adr-rt3. 16x16 cell and hero animated in code (2026-11-02)

**Decision.** The grid cell is 16x16 art pixels. The hero is pack character 9 (14x16 px). The pack has one static pose per character, so the hero is animated in code: left and right flip, a small bob while walking, a squash when idle.

**Rationale.** The pack tileset is a 16 px grid and the hero is 16 px tall, so one cell fits one hero. Drawing four-direction walk cycles costs art time that the first playable does not need. The Aseprite tags `Idle`, `Walk`, `Up`, `Down` that Long Way uses are not available for these sprites.

**How to apply.** Replace the code animation with drawn frames later without changing the hero's size or collision. The camera view height must keep an integer scale at a 16 px cell (open question in `SPEC.md`).

## adr-rt4. 240 px view height and a 60x60 town (2026-11-02, map size changed the same day)

**Decision.** The camera shows 240 art pixels of height, which is 15 cells of 16 px. The first town map is 60x60 cells (it was 80x60 at first, and became a blank 60x60 map filled in by hand, `adr-rt7`). Water blocks the hero.

**Rationale.** 240 is exactly 15 cells, so the view always holds whole cells vertically. It scales by a whole number on 720p (3x), 1440p (6x) and 2160p (9x). On 1080p the scale is 4.5, not a whole number. Fractional scale is accepted, as in Long Way, so the view stays exactly 15 cells tall without letterboxing. 60x60 cells is about two screens wide and four tall, enough to need a scrolling camera.

Walk speed is 80 art pixels per second (5 cells per second). Houses and trees block the hero, as water does.

**How to apply.** Set the view height once in the R-Town camera module and pass it to the kit camera helper (adr-x12). Cell size 16 comes from adr-rt3.

## adr-rt5. Collision rules for the first town (2026-11-02)

**Decision.** Water, houses, trees, walls, fences, wells, barrels, crates and chests block the hero. Houses and trees block at the base only, so the hero walks behind the top (the Long Way trunk rule, adr-lw1). Other objects block their whole tile. The hero's feet box is 10x4 px.

**Rationale.** The base-only rule keeps the forgiving feel of Long Way and lets the hero pass behind roofs and leaves. A 10 px wide box fits through a one-cell (16 px) gap with 3 px to spare each side. A 4 px tall box gives a clear walk-behind line.

**How to apply.** The kit collision takes these sizes as values from the game (adr-x12). Base footprint sizes per kind go in the R-Town map kind table.

## adr-rt6. Art file layout, tags and map conventions (2026-11-02)

**Decision.** Pack PNGs are packed by `dev-scripts/aseprite/pack_pngs.lua` into `assets/art/characters/characters.aseprite` (tags `char_<n>`) and `assets/art/features/background-elements.aseprite` (tags `bg_<n>`). Files are named after the pack folder, not the category names in adr-rt1. Images sit at the bottom centre of a shared canvas. The catalogue of frames and tags is `games/r-town/docs/art-tags.md`, in `docs/` and not in `assets/`, so it is not shipped in a web build. A map object stands on its cell: the bottom centre of its art is the bottom centre of the cell. The hero entity is at the feet, with the sprite as a child. The map size is the `size` key in the map file (`adr-rt7`), and cell (0, 0) is at the world origin.

**Rationale.** A shared canvas with a common bottom line means one anchor rule for every sprite, and feet or tree bases never need per-sprite offsets. A child sprite lets the walking bob move the picture and not the collision feet. The tags are the contract with code, and a catalogue next to the ADRs keeps developer material out of shipped assets.

**How to apply.** Add a sprite by adding its PNG to the pack command and a row to `art-tags.md`. Measure its base from the bottom rows of the PNG before using it as an obstacle.

## adr-rt7. R-Town map: top-left grid, size and hero start in the file (2026-11-02)

**Decision.** `assets/maps/test.txt` uses `origin: top-left`, so row labels count down the page from 0, like rows of text. The file holds the map size (`size: 60x60`) and the hero start, a legend marker `"@": player_start` at file cell (10, 2). The code has no `MAP_CELLS` constant, and the hero spawns at the marker cell. The F3 grid numbers rows from the top to match the file (`DebugSettings::rows_from_top`).

**Rationale.** The developer wanted to read the map like a page, with cell (0, 0) at the top left. The size and the start belong to the map, so they live in the file. The parser still returns cells numbered from the bottom left with y up, so collision, camera and spawn code did not change.

**How to apply.** Read cell numbers from the file for editing, and from the F3 grid for play. Both use the same numbers. A marker has no art and no `Kind`: `load_map` in `map.rs` removes it from the object list. The file needs exactly one `player_start`. The format is in `dev-docs/extensions/map-files.md` (`adr-x18`).

## adr-rt8. Map file is read from disk and reloads on F1 (2026-11-03)

**Decision.** A native run reads the map file from disk, not from `include_str!`. `./dev-scripts/rust.sh run r-town [map]` takes a path or a name in `assets/maps/` and passes it in `R_TOWN_MAP`. With no argument the game reads `assets/maps/test.txt` through `CARGO_MANIFEST_DIR`. F1 reads the file again and rebuilds the ground, trees, collision, camera limits and grid row numbers. The hero stays where it is. A file with an error is logged and the current map stays. The web build and a binary with no environment use the bundled copy, so F1 has no new content there.

**Rationale.** Edit the map in a text editor and see it at once, without a rebuild. This is the native half of roadmap item 3. The hero stays so the developer can check the same spot after an edit.

**How to apply.** Everything a map spawns has the `MapEntity` component, so a reload removes it. A new map system must tag its entities with it and be run in `reload_map` (`src/map.rs`).

## adr-rt9. A lone item blocks a band, a group blocks its cells (2026-11-03)

**Decision.** This amends the base-only rule of `adr-rt5`. An item with no blocking neighbour in the 8 cells around it blocks only its base band, so the hero walks behind it. Items that touch, also at a corner and also of different kinds, form a group, and each member blocks its whole 16 px cell grown sideways by the feet half width. The members merge into one cluster with one outline (`adr-x10`). The rule applies to every blocking kind, and a new kind must be counted in `blocking_group_sizes` (`src/map.rs`). The kit finds the groups (`adr-x20`).

**Rationale.** The developer wants a single tree to be passable from behind and a grove to be a solid wall with an outline, like the rock piles of Long Way. With a 4 px band the rows of a grove, 16 px apart, left 12 px gaps, so the group was one cluster per row.

**How to apply.** Edit the map and press F1 to see the result. A reload recomputes the groups. The footprint rule is to be re-assessed after play testing (roadmap).
