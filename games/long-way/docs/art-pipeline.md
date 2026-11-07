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
- `diodrama_palette.aseprite` (from `diodrama_palette.aseprite`, 19 colours, expanded to 32 on 2026-11-05 with the 13 Hunter colours, see `adr-a5`). The game's palette.
- `characters/Guy.aseprite` (from `characters/guy/Guy.aseprite`, 60x60, 41 frames, tags: Idle, Walk, Up, Down, Block, Win)
- `characters/Hunter.aseprite` (built from the Clarice "nocturnal creature hunter" sprite pack in `input/hunter/`, which the user has a full commercial licence for and may edit. 128x128 (doubled with `resize.lua`), 46 frames, tags: Idle, Run, Hurt, Die, Up and Down. Up and Down were redrawn by the user (2026-11-05). Layers: Body, Weapon, background (never drawn). Built with `dev-scripts/aseprite/tagged_sheets.lua`. Details in `HUNTER-sprites.md`. On the game palette.)
- `features/Green tree.aseprite` (from `features/`, resized in our copy from 512x512 to 128x128 with `dev-scripts/aseprite/resize.lua`, 8 frames, no tags, 12 colour palette, not the game palette)
- `features/Red tree.aseprite` (from `features/`, resized in our copy from 64x64 to 128x128 with `dev-scripts/aseprite/resize.lua`, 8 frames, no tags, 6 colour palette, not the game palette)
- `features/Rocks.aseprite` (built from `features/sprites/rock_0.png` and `rock_1.png`, both 256x256 (32x32 art scaled 8x), with `dev-scripts/aseprite/frames_from_pngs.lua`: native 32x32, 2 frames, drawn in the game at a whole number scale (4x = 128 px), tags `rock_0` and `rock_1`, own colour palette, not the game palette). The PNGs are not kept in the repo.
- `features/Markers.aseprite` (built from `input/lantern.png` and `input/lantern_red.png` (160x200, with a glow) and `input/marker.png` and `input/marker_red.png` (120x196), with `dev-scripts/aseprite/frames_from_pngs.lua` at `width=28 height=35` (nearest neighbour, each PNG resized on its own, so the markers are stretched a little wider): 10 frames, tags `lantern` (1 frame), `lantern_red` (1 frame), `marker` (4 frames) and `marker_red` (4 frames). The marker frames last 250 ms each and loop (set with `set_durations`). Renamed from `Lanterns.aseprite`; drawn at scale 1, 70% of the player's body height, own colour palette, not the game palette). The PNGs stay in `input/`.

## Pipeline

Source of truth: `.aseprite` files in each game's `assets/art/` (`games/<name>/assets/art/`). Exported files go in `games/<name>/assets/generated/` and are generated, not edited by hand. Lua scripts in `dev-scripts/aseprite/` are shared by all games and take the game as an argument.

Tools:
- `./dev-scripts/aseprite.sh` runs `check`, `frame_grid`, `sheet_analysis` and `crop` (`dev-docs/extensions/dev-scripts.md`).
- Aseprite CLI, run as `aseprite --batch --script <script.lua>`. Binary: `/Applications/Aseprite.app/Contents/MacOS/aseprite`.
- Lua scripts in `dev-scripts/aseprite/`. Claude writes and runs them. `pack_pngs.lua` packs PNGs of different sizes into one file (bottom centre of a shared canvas, one tag per image). `frames_from_pngs.lua` needs equal sizes, or a `width` and `height` to resize every PNG to. `resize.lua` resizes. `tagged_sheets.lua` builds one file from several sprite sheets (one tag per sheet, durations from the Aseprite JSON, body re-centred on a new canvas).

Rules:
- The art never sets game mechanics. Feet, hitbox and collision come from constants in code (Long Way: `PLAYER_FEET_OFFSET`, `PLAYER_HALF_WIDTH`, `PLAYER_BODY_HEIGHT` in `collision.rs`), not from the pixels. Moving or bobbing a sprite inside a frame is a visual change only. The `check` script reports bounds and a feet row from the pixels, so read those as a visual check, not a game value.
1. Write one shared helper first (`dev-scripts/aseprite/lib.lua`): new sprite, load palette, draw tile, outline, shade, tag frames, save. Sprite scripts call the helper. Do not repeat drawing code per sprite.
2. Use one fixed palette per game. Long Way: `games/long-way/assets/art/diodrama_palette.aseprite` (32 colours, index 0 is transparent black `#000000a0`). Scripts must not use colours outside it.
3. Use one tile and sprite size grid (open question: size not chosen, for example 16x16 or 32x32). All sprites are multiples of it.
4. Check each sprite visually: export a PNG scaled up with `--scale`, view it, fix, repeat. Stop after 3 rounds and ask the user if it still looks wrong.
5. Claude draws placeholders, tiles, icons, UI, effects and simple animation. The user draws or refines characters and key art. Mark placeholders with the suffix `_placeholder` in the file name so they are easy to find.
6. Load `.aseprite` files in the game through `bevy_aseprite_ultra` (verify it supports the pinned Bevy version). If it does not, export PNG sheets and JSON with `--sheet` and `--data` and load those.
7. Use layers and animation tags as the contract between art and code. Keep a list of tag names in `art-tags.md`.
8. Animation speed is set in the `.aseprite` file, as frame durations, and not in code. Do not call `Animation::with_speed` to slow down or speed up a tag. See "Animation speed" in `dev-docs/extensions/pixel-art.md` and `adr-a6`.
