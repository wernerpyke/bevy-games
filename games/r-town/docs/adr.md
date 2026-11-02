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

## adr-rt4. 240 px view height and an 80x60 town (2026-11-02)

**Decision.** The camera shows 240 art pixels of height, which is 15 cells of 16 px. The first town map is 80x60 cells. Water blocks the hero.

**Rationale.** 240 is exactly 15 cells, so the view always holds whole cells vertically. It scales by a whole number on 720p (3x), 1440p (6x) and 2160p (9x). On 1080p the scale is 4.5, not a whole number. Fractional scale is accepted, as in Long Way, so the view stays exactly 15 cells tall without letterboxing. 80x60 cells is about three screens wide and four tall, enough to need a scrolling camera.

Walk speed is 80 art pixels per second (5 cells per second). Houses and trees block the hero, as water does.

**How to apply.** Set the view height once in the R-Town camera module and pass it to the kit camera helper (adr-x12). Cell size 16 comes from adr-rt3.

## adr-rt5. Collision rules for the first town (2026-11-02)

**Decision.** Water, houses, trees, walls, fences, wells, barrels, crates and chests block the hero. Houses and trees block at the base only, so the hero walks behind the top (the Long Way trunk rule, adr-lw1). Other objects block their whole tile. The hero's feet box is 10x4 px.

**Rationale.** The base-only rule keeps the forgiving feel of Long Way and lets the hero pass behind roofs and leaves. A 10 px wide box fits through a one-cell (16 px) gap with 3 px to spare each side. A 4 px tall box gives a clear walk-behind line.

**How to apply.** The kit collision takes these sizes as values from the game (adr-x12). Base footprint sizes per kind go in the R-Town map kind table.

## adr-rt6. Art file layout, tags and map conventions (2026-11-02)

**Decision.** Pack PNGs are packed by `dev-scripts/aseprite/pack_pngs.lua` into `assets/art/characters/characters.aseprite` (tags `char_<n>`) and `assets/art/features/background-elements.aseprite` (tags `bg_<n>`). Files are named after the pack folder, not the category names in adr-rt1. Images sit at the bottom centre of a shared canvas. The catalogue of frames and tags is `games/r-town/docs/art-tags.md`, in `docs/` and not in `assets/`, so it is not shipped in a web build. A map object stands on its cell: the bottom centre of its art is the bottom centre of the cell. The hero entity is at the feet, with the sprite as a child. The test map has no size key: the size is `MAP_CELLS` in `src/map.rs` (80x60), and cell (0, 0) is at the world origin.

**Rationale.** A shared canvas with a common bottom line means one anchor rule for every sprite, and feet or tree bases never need per-sprite offsets. A child sprite lets the walking bob move the picture and not the collision feet. The tags are the contract with code, and a catalogue next to the ADRs keeps developer material out of shipped assets.

**How to apply.** Add a sprite by adding its PNG to the pack command and a row to `art-tags.md`. Measure its base from the bottom rows of the PNG before using it as an obstacle.
