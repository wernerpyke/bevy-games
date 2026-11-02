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
- `diodrama_palette.aseprite` (from `diodrama_palette.aseprite`, 19 colours). The game's palette.
- `characters/Guy.aseprite` (from `characters/guy/Guy.aseprite`, 60x60, 41 frames, tags: Idle, Walk, Up, Down, Block, Win)
- `features/Green tree.aseprite` (from `features/`, resized in our copy from 512x512 to 128x128 with `dev-scripts/aseprite/resize.lua`, 8 frames, no tags, 12 colour palette, not the game palette)
- `features/Red tree.aseprite` (from `features/`, resized in our copy from 64x64 to 128x128 with `dev-scripts/aseprite/resize.lua`, 8 frames, no tags, 6 colour palette, not the game palette)
- `features/Rocks.aseprite` (built from `features/sprites/rock_0.png` and `rock_1.png`, both 256x256 (32x32 art scaled 8x), with `dev-scripts/aseprite/frames_from_pngs.lua`: native 32x32, 2 frames, drawn in the game at a whole number scale (4x = 128 px), tags `rock_0` and `rock_1`, own colour palette, not the game palette). The PNGs are not kept in the repo.

## Pipeline

Source of truth: `.aseprite` files in each game's `assets/art/` (`games/<name>/assets/art/`). Exported files go in `games/<name>/assets/generated/` and are generated, not edited by hand. Lua scripts in `dev-scripts/aseprite/` are shared by all games and take the game as an argument.

Tools:
- Aseprite CLI, run as `aseprite --batch --script <script.lua>`. Binary: `/Applications/Aseprite.app/Contents/MacOS/aseprite`.
- Lua scripts in `dev-scripts/aseprite/`. Claude writes and runs them.

Rules:
1. Write one shared helper first (`dev-scripts/aseprite/lib.lua`): new sprite, load palette, draw tile, outline, shade, tag frames, save. Sprite scripts call the helper. Do not repeat drawing code per sprite.
2. Use one fixed palette per game. Long Way: `games/long-way/assets/art/diodrama_palette.aseprite` (19 colours, index 0 is transparent black `#000000a0`). Scripts must not use colours outside it.
3. Use one tile and sprite size grid (open question: size not chosen, for example 16x16 or 32x32). All sprites are multiples of it.
4. Check each sprite visually: export a PNG scaled up with `--scale`, view it, fix, repeat. Stop after 3 rounds and ask the user if it still looks wrong.
5. Claude draws placeholders, tiles, icons, UI, effects and simple animation. The user draws or refines characters and key art. Mark placeholders with the suffix `_placeholder` in the file name so they are easy to find.
6. Load `.aseprite` files in the game through `bevy_aseprite_ultra` (verify it supports the pinned Bevy version). If it does not, export PNG sheets and JSON with `--sheet` and `--data` and load those.
7. Use layers and animation tags as the contract between art and code. Keep a list of tag names in `art-tags.md`.
