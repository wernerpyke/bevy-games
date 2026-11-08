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

## adr-a4. Project palette: `game.aseprite` (was `diodrama_palette.aseprite`) (2026-10-31)

**Decision.** The Long Way palette is `games/long-way/assets/art/game.aseprite` (named `diodrama_palette.aseprite` until 2026-11-08) (19 colours, copied from the legacy library). Each game chooses its own palette. The 20-colour `characters/guy/palette.aseprite` is not used.

**Rationale.** It is the palette embedded in `Guy.aseprite` and in most of the legacy character files, so existing art needs no recolouring.

**How to apply.** New sprites and Lua scripts use only these colours. Other legacy files with different palettes (for example the 32-colour environment files) need remapping when they are copied across.

## adr-a5. The Long Way palette may grow, by appending (2026-11-05)

**Decision.** The palette in `game.aseprite` is expanded when new art needs colours. New colours are added at the end. Existing indexes never move. The first expansion added the 13 colours of the Hunter sprite pack (indexes 19 to 31, 32 colours in total). `dev-scripts/aseprite/extend_palette.lua` does it.

**Rationale.** The Hunter pack uses its own colours. Mapping them to the 19 old colours would change the look of the character. A bigger palette keeps the pack art as drawn. Appending keeps the player, the rocks and the trees unchanged, because sprites refer to colours, not to index numbers, only inside Aseprite.

**How to apply.** Run the script with the palette and the sprite that needs the colours. It adds only colours that are not yet in the palette. Update the colour count in `art-pipeline.md`. This amends `adr-a4`: "only these colours" now means "only colours in the current palette".

## adr-a6. Animation speed is set as frame durations in the `.aseprite` file (2026-11-07)

**Decision.** How fast a fixed animation plays is set in the `.aseprite` file, as the duration of each frame. Code does not scale it with `Animation::with_speed`. The loop is the library default. The markers of Long Way use 250 ms per frame. `speed` stays allowed for a rate that follows the game state, such as a run animation tied to the walk speed.

**Rationale.** The file is the source of truth for art (`art-pipeline.md`). A speed factor in code only works while every frame has the duration the code assumes, and nothing warns when the art changes. In the file the timing can differ per frame and per tag, and the Aseprite preview shows what the game will show. We first set the speed in code (0.4 times the 100 ms of the file) and moved it into the file the same day.

**How to apply.** Set durations in Aseprite or with `dev-scripts/aseprite/set_durations.lua` (`apply=1` writes), and read them with `check`. Rules in `dev-docs/extensions/pixel-art.md`, "Animation speed".

## adr-a7. Tile sheets are catalogued at 1:1 and cut by position, not used as pre-cut tiles (2026-11-07)

**Decision.** A bought tile sheet is turned into an `.aseprite` catalogue (`dev-scripts/aseprite/catalogue.lua`): the sheet at 1:1 with a column and row ruler in a margin, a grid and a backdrop layer. A sprite is requested by its top left column and row and its size in tiles, and Claude cuts it from the original sheet one sprite at a time. The numbered single-tile folder of a pack (the Zen Garden `Tiles/` folder) is not used. The source PNG is never scaled. Scaling happens on a single extracted sprite, in Aseprite.

**Rationale.** The numbered tiles (587 files for Zen Garden) have no coordinates and split an object such as a pagoda over many files, so a person cannot find or name anything. The sheet shows objects whole, and the rulers give the person a way to point at them. A scaled copy of the full PNG adds a second source that can drift from the original. The sheet background in Zen Garden was already transparent, so the catalogue only needs a backdrop layer to show it.

**How to apply.** Rules and usage in `dev-docs/extensions/tile-sheet-catalogue.md`. Record each extracted sprite in `games/<game>/docs/art-pipeline.md`.

## adr-a8. Pack art is nudged onto the game palette, and a colour is added only when nothing close exists (2026-11-07)

**Decision.** Sprites taken from a bought pack are moved onto the game palette. A colour that has a close palette colour is replaced by it. A colour with no close match is appended to the palette (`adr-a5`) instead of being forced onto a wrong colour. The first use was the five Zen Garden trees and shrubs: 8 colours were mapped, 4 were added (36 colours in total), and the pack's `#4b6396` was merged into `#425faa`.

**Rationale.** The game palette is mostly grey, with one bright green. A strict nearest-colour mapping turned the green trees grey and the blue shadows into one bright cyan, which lost the look of the art. Adding every pack colour would grow the palette with near duplicates and would not align the art at all. The hybrid keeps the look and moves the outline, greys, light green and browns onto the colours the game already has. Compared on a side by side preview before the choice.

**How to apply.** Compare the colours of the sprite with the palette first. Replace the close matches with `dev-scripts/aseprite/remap_colors.lua` (dry run until `apply=1`). Add the rest with `extend_palette.lua`, then run `check palette=` to confirm no colour is outside the palette. A pair is "close" when it looks like the same colour in a side by side preview, not by a fixed distance. Update the colour count in `art-pipeline.md`.
