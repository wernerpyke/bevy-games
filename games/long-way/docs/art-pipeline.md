# Art pipeline

## Existing asset library (copy only on request)

Starting-point art lives in `~/Dark/graveyard_shift/diodrama-assets/` (55 `.aseprite` files, plus `.xcf`, `.tps` and `.jpg` files). It is a read-only source library.

- **Do NOT copy, import, convert or reference any file from it until the user names that file.** The user will say which file to copy across and when. Do not bulk copy the folder, and do not copy "related" files (palettes, other animations, `.xcf`, `.tps`) on your own initiative.
- Never modify or delete anything in the source library. Copy only.
- When the user names a file, ask which game it is for if that is not clear, copy it into that game's `assets/art/` (characters in `assets/art/characters/`, scenery in `assets/art/features/`), keeping its file name, then record it in the list below.
- Browsing the library to answer a question is allowed. Reading a file with the Aseprite CLI to inspect it is allowed. Writing outside the project is not.
- Licence: the game is for personal use. The user owns a licence for "Forest Graveyard" (GuttyKreum, GameDev Market), the source of the environment sprites. No redistribution is planned.

## Files copied so far

Long Way (`games/long-way/assets/art/`):
- `game.aseprite` (the palette; named `diodrama_palette.aseprite` until 2026-11-08; from the legacy `diodrama_palette.aseprite`, 19 colours, expanded to 32 on 2026-11-05 with the 13 Hunter colours, to 36 on 2026-11-07 with 4 Zen Garden colours (indexes 32 to 35: `#508582`, `#425faa`, `#314b8e`, `#e78d7b`), to 44 on 2026-11-08 with 8 colours of the HUD background (indexes 36 to 43: `#1f1932`, `#78b9db`, `#5677af`, `#5285ac`, `#232f50`, `#415b94`, `#386764`, `#2a3f5b`), to 45 with `#bb6479` (index 44) of the red trees, and to 55 on 2026-11-08 (`adr-a9`: indexes 45 to 54 `#baa186`, `#2d3554`, `#35405f`, `#83374f`, `#7099d1`, `#3d4a6e`, `#3b3d65`, `#755c4f`, `#204c73`, `#5b283b`; index 38 is now `#5e77af` and index 26 `#c5c2c5`), see `adr-a5` and `adr-a8`). The game's palette.
- `fonts/CourierPrime-Regular.ttf` and `fonts/OFL.txt` (Courier Prime, copyright 2015 The Courier Prime Project Authors, SIL Open Font Licence 1.1, from the Google Fonts repository on 2026-11-07, not edited. The licence text ships next to the font. Used by the HUD text, `adr-lw24`. Not drawn art: it is not on the palette and not pixel art.)
- `characters/Hunter.aseprite` (built from the Clarice "nocturnal creature hunter" sprite pack in `input/hunter/`, which the user has a full commercial licence for and may edit. 128x128 (doubled with `resize.lua`), 46 frames, tags: Idle, Run, Hurt, Die, Up and Down. Up and Down were redrawn by the user (2026-11-05). Layers: Body, Weapon, background (never drawn). Built with `dev-scripts/aseprite/tagged_sheets.lua`. Details in `HUNTER-sprites.md`. On the game palette.)
- `features/Green tree.aseprite` (from `features/`, resized in our copy from 512x512 to 128x128 with `dev-scripts/aseprite/resize.lua`, 8 frames, no tags, now on the game palette since 2026-11-08, see `adr-a9`)
- `features/Red tree.aseprite` (from `features/`, resized in our copy from 64x64 to 128x128 with `dev-scripts/aseprite/resize.lua`, 8 frames, no tags, now on the game palette since 2026-11-08, see `adr-a9`)
- `features/Rocks.aseprite` (built from `features/sprites/rock_0.png` and `rock_1.png`, both 256x256 (32x32 art scaled 8x), with `dev-scripts/aseprite/frames_from_pngs.lua`: native 32x32, 2 frames, drawn in the game at a whole number scale (4x = 128 px), tags `rock_0` and `rock_1`, now on the game palette since 2026-11-08, see `adr-a9`). The PNGs are not kept in the repo.
- `features/Markers.aseprite` (built from `input/lantern.png` and `input/lantern_red.png` (160x200, with a glow) and `input/marker.png` and `input/marker_red.png` (120x196), with `dev-scripts/aseprite/frames_from_pngs.lua` at `width=28 height=35` (nearest neighbour, each PNG resized on its own, so the markers are stretched a little wider): 10 frames, tags `lantern` (1 frame), `lantern_red` (1 frame), `marker` (4 frames) and `marker_red` (4 frames). The marker frames last 250 ms each and loop (set with `set_durations`). Renamed from `Lanterns.aseprite`; drawn at scale 1, 70% of the player's body height, own colour palette, not the game palette). The PNGs stay in `input/`.
- `features/green_tree_medium.aseprite`, `red_tree_medium.aseprite`, `red_tree_small.aseprite`, `green_shrub.aseprite`, `green_shrub_small.aseprite` (Zen Garden `Main_Tilemap.png`, cut on 2026-11-07 with `dev-scripts/aseprite/extract_block.lua`, 1 frame each, no tags, on the game palette (nudged with `remap_colors.lua`, see `adr-a8`), scaled 2x with nearest neighbour so 1 tile (32 px) is 1 cell (64 px)). Source block (col, row, size in tiles) and result:
  - `green_tree_medium`: c20 r4, 2x2, `anchor=bottom`, 128x128. Re-cut on 2026-11-07: the art keeps its horizontal place in the 2x2 block and the canvas has its bottom left corner on the cell, like `green_tree` (first cut centred the trunk by mistake).
  - `red_tree_medium`: c23 r6, 1x2, `anchor=trunk`, 84x128. The crown is wider than the 1x2 cell space, so the canvas is widened (64 to 84 px) and the trunk foot is on the middle column of the canvas.
  - `red_tree_small`: c22 r4, 1x1, `anchor=bottom`, 64x64. The lowest pixel row is the bottom of the cell.
  - `green_shrub`: c22 r5, 1x1, `anchor=bottom`, 64x64.
  - `green_shrub_small`: c23 r5, 1x1, `anchor=bottom`, 64x64.
  The game draws `green_tree_medium` with its canvas bottom left on the cell and the other four with the bottom middle of the canvas on the bottom middle of their cell space (`scenery.rs`).
- `pickups/pickups.aseprite` (built on 2026-11-07 from the user's `input/pickups.aseprite`, 66x317, 8 frames with a blue, a red and a violet fireball stacked in each frame. Brown-grey stray pixels (palette indexes 1 to 8 plus 5 near duplicates) and the pale diagonal streaks (11 blended colours, healed to the neighbouring ramp colour) were removed. Cut with no scaling into 64x96 cells (each fireball is at most 64x95 px and touches the canvas edge), 24 frames, tags `pickup_blue`, `pickup_red` and `pickup_violet` (8 frames each, 100 ms per frame). One layer `Pickup`. 13 colours (3 ramps of 5 colours sharing one white core), own palette, not the game palette. `input/` is git-ignored, so the source and its backup are not in the repo. Not used in the game yet.)
- `backgrounds/hud.aseprite` (made on 2026-11-08 with `dev-scripts/aseprite/new_sprite.lua`: 512x160 art pixels, the HUD bar size at 5/16 of the view height (`adr-lw24`), 1 frame, one layer `Background` filled with palette index 34 (`#314b8e`), game palette. Drawn at 1 art pixel per UI pixel (`UiScale`), so it is as wide as the game view. A blank start for the user to draw on.)

## Zen Garden pack (`input/zen-garden/`)

The user has a commercial licence for the GuttyKreum Zen Garden pack, for Long Way only. Use `Non-RPGMaker_Tilemap/Main_Tilemap.png` (32x32 tiles, transparent background) through `Main_Tilemap_catalogue.aseprite`, and the named files in `AnimationedTiles/`. Do not use the numbered `Tiles/` folder or the `RPGMakerVXAce/` sheets (`adr-a7`). The user finds a sprite in the catalogue and gives its top left column and row and its size in tiles. Method and rules: `dev-docs/extensions/tile-sheet-catalogue.md`. Extracted sprites are listed under "Files copied so far" with their source position.

## Pipeline

Source of truth: `.aseprite` files in each game's `assets/art/` (`games/<name>/assets/art/`). Exported files go in `games/<name>/assets/generated/` and are generated, not edited by hand. Lua scripts in `dev-scripts/aseprite/` are shared by all games and take the game as an argument.

Tools:
- `./dev-scripts/aseprite.sh` runs `check`, `frame_grid`, `sheet_analysis` and `crop` (`dev-docs/extensions/dev-scripts.md`).
- Aseprite CLI, run as `aseprite --batch --script <script.lua>`. Binary: `/Applications/Aseprite.app/Contents/MacOS/aseprite`.
- Lua scripts in `dev-scripts/aseprite/`. Claude writes and runs them. `pack_pngs.lua` packs PNGs of different sizes into one file (bottom centre of a shared canvas, one tag per image). `frames_from_pngs.lua` needs equal sizes, or a `width` and `height` to resize every PNG to. `resize.lua` resizes. `tagged_sheets.lua` builds one file from several sprite sheets (one tag per sheet, durations from the Aseprite JSON, body re-centred on a new canvas).

Rules:
- The art never sets game mechanics. Feet, hitbox and collision come from constants in code (Long Way: `PLAYER_FEET_OFFSET`, `PLAYER_HALF_WIDTH`, `PLAYER_BODY_HEIGHT` in `collision.rs`), not from the pixels. Moving or bobbing a sprite inside a frame is a visual change only. The `check` script reports bounds and a feet row from the pixels, so read those as a visual check, not a game value.
1. Write one shared helper first (`dev-scripts/aseprite/lib.lua`): new sprite, load palette, draw tile, outline, shade, tag frames, save. Sprite scripts call the helper. Do not repeat drawing code per sprite.
2. Use one fixed palette per game. Long Way: `games/long-way/assets/art/game.aseprite` (55 colours, index 0 is transparent black `#000000a0`). Scripts must not use colours outside it.
3. Use one tile and sprite size grid (open question: size not chosen, for example 16x16 or 32x32). All sprites are multiples of it.
4. Check each sprite visually: export a PNG scaled up with `--scale`, view it, fix, repeat. Stop after 3 rounds and ask the user if it still looks wrong.
5. Claude draws placeholders, tiles, icons, UI, effects and simple animation. The user draws or refines characters and key art. Mark placeholders with the suffix `_placeholder` in the file name so they are easy to find.
6. Load `.aseprite` files in the game through `bevy_aseprite_ultra` (verify it supports the pinned Bevy version). If it does not, export PNG sheets and JSON with `--sheet` and `--data` and load those.
7. Use layers and animation tags as the contract between art and code. Keep a list of tag names in `art-tags.md`.
8. Animation speed is set in the `.aseprite` file, as frame durations, and not in code. Do not call `Animation::with_speed` to slow down or speed up a tag. See "Animation speed" in `dev-docs/extensions/pixel-art.md` and `adr-a6`.
