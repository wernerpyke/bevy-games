# Art pipeline: decisions

## adr-a1. `.aseprite` files are the art source of truth (2026-10-30)

**Decision.** Store art as `.aseprite` files in each game's `assets/art/` (`games/<name>/assets/art/`). Exported files are generated and never edited by hand.

**Rationale.** Aseprite is the developer's preferred editor. It has a CLI and a Lua API, so Claude can generate and export sprites by script. Hot reload through `bevy_aseprite_ultra` (if it supports Bevy 0.20) keeps iteration fast.

**How to apply.** Claude draws placeholders, tiles, icons, UI and simple animation by Lua script. The developer draws or refines characters and key art. Placeholder files carry the suffix `_placeholder`. Layers and animation tags are the contract between art and code.

## adr-a2. Lua scripts through the Aseprite CLI (2026-10-30)

**Decision.** Drive Aseprite with `aseprite --batch --script`. Scripts live in `dev-scripts/aseprite/` and share one helper, `lib.lua`.

**Rationale.** One shared helper avoids repeating drawing code per sprite and keeps token cost low.

**How to apply.** Sprites use one fixed palette and one tile size grid. Both are still to be chosen (see `ROADMAP.md`).

## adr-a3. Existing assets are copied one file at a time, on request (2026-10-31)

**Decision.** The legacy library `~/Dark/graveyard_shift/diodrama-assets/` stays a read-only source. A file enters a game's `assets/art/` only when the developer names it. The first file is `Guy.aseprite`.

**Rationale.** The library has 55 Aseprite files plus `.xcf`, `.tps` and `.jpg` files with two different palettes and no common size grid. Importing everything would carry unused and inconsistent art into the project. Choosing files one at a time keeps the project small and lets palette and size decisions follow real needs.

**How to apply.** Claude never copies a file from the library without being told which one. After a copy, list the file in `games/long-way/docs/art-pipeline.md` under "Files copied so far", under the game it belongs to. The game is for personal use and the developer owns a licence for "Forest Graveyard", the source of the environment sprites, so redistribution is not a concern.

## adr-a4. Project palette: `diodrama_palette.aseprite` (2026-10-31)

**Decision.** The Long Way palette is `games/long-way/assets/art/diodrama_palette.aseprite` (19 colours, copied from the legacy library). Each game chooses its own palette. The 20-colour `characters/guy/palette.aseprite` is not used.

**Rationale.** It is the palette embedded in `Guy.aseprite` and in most of the legacy character files, so existing art needs no recolouring.

**How to apply.** New sprites and Lua scripts use only these colours. Other legacy files with different palettes (for example the 32-colour environment files) need remapping when they are copied across.

## adr-a5. The Long Way palette may grow, by appending (2026-11-05)

**Decision.** The palette in `diodrama_palette.aseprite` is expanded when new art needs colours. New colours are added at the end. Existing indexes never move. The first expansion added the 13 colours of the Hunter sprite pack (indexes 19 to 31, 32 colours in total). `dev-scripts/aseprite/extend_palette.lua` does it.

**Rationale.** The Hunter pack uses its own colours. Mapping them to the 19 old colours would change the look of the character. A bigger palette keeps the pack art as drawn. Appending keeps the player, the rocks and the trees unchanged, because sprites refer to colours, not to index numbers, only inside Aseprite.

**How to apply.** Run the script with the palette and the sprite that needs the colours. It adds only colours that are not yet in the palette. Update the colour count in `art-pipeline.md`. This amends `adr-a4`: "only these colours" now means "only colours in the current palette".
