# PLAN: text maps

Status: in progress, steps 1 to 7 done and step 8 under way. Date: 2026-11-02.

Goal: one text file defines a whole map as a drawn grid. Replace the markdown format (`# Features` and `# Map` sections, adr-x11) in `game-kit`, Long Way and R-Town. Background and options: `dev-docs/RESEARCH-map-layouts.md`.

Draft files already written (not read by the game yet): `games/long-way/assets/maps/rocks-test.txt` and `games/r-town/assets/maps/test.txt`.

## Decisions already made

| Topic | Decision |
|---|---|
| Layers | One grid, one object per cell. No layers now. |
| Multi-cell objects | One character marks the anchor cell (bottom-left). The game knows the size. |
| Notes | Lines that start with `#` are ignored. |
| Row alignment | Rulers and a size check. |
| Large maps | Out of scope. |
| Front matter | Small hand parser, no new dependency. |
| Blank cell | A blank means empty, the same as `.`. People may draw a footprint with `.` or with blanks. |
| Rulers | Zero-based, same numbers as the F2 grid. Row labels are checked. Bottom ruler is optional. |
| Seeds | Later. The parser returns a plain `Map`, so a generator can return the same type. |

## Decisions to confirm while reviewing this plan

1. **Map size comes from the file.** `size: 80x60` replaces `MAP_CELLS` in `games/r-town/src/map.rs`. The `Map` struct gets a `size` field. Recommended, because the size is part of the drawing.
2. **Markers are ordinary objects.** `"@": player_start` returns an object with kind `player_start`. The game reads it for the spawn cell and does not spawn art for it. R-Town replaces its centre constant with this cell. Long Way has no spawn marker.
3. **`pitch` is removed.** Cells always sit two characters apart now. `draw` and `jitter` stay as optional keys.

## Format (summary, full text goes to `dev-docs/extensions/map-files.md`)

```
---
map: test
cell: 16
origin: bottom-left
size: 80x60
legend:
  ".": empty
  "@": player_start
  "g": tree_big
---
   |0                   1
   |0 1 2 3 4 5 6 7 8 9 0 1 ...
59 |. . . . . . . . . . . . ...|
 0 |. . . . . . . . . . . . ...|
```

1. Front matter sits between two `---` lines. It has `map`, `cell`, `origin` (`bottom-left` or `top-left`), `size` (`WxH`) and `legend`. It may have `draw` and `jitter`.
2. After the second `---`, every line is one of: a comment (starts with `#`), a blank line, a ruler line, or a grid row. Blank lines and comments are ignored.
3. A grid row is `<label>|<cells>|`. The label is the row number, right-aligned. With `bottom-left` rows run from `H-1` at the top to 0 at the bottom. With `top-left` they run from 0 at the top to `H-1`. `parse` always returns cells numbered from the bottom left.
4. A ruler line has a blank label. It is ignored. Rulers may be at the top, at the bottom, or both.
5. Cells sit between the first and the last `|`. Cell k is the character at offset 2k. The characters at odd offsets must be spaces. The cell part is exactly `2W-1` characters long.
6. A blank cell or `.` is empty. Any other character must be in the legend. The legend cannot use `.`, `|` or a space.
7. The anchor of every object is its cell. Footprint size is not in the file.
8. Errors stop loading and name the file and line.

## Test cases

Parser tests in `crates/game-kit/src/map.rs`. The first eight are the agreed list.

1. A tree with a dotted footprint and a tree with a blank footprint give the same object list.
2. One grid that mixes both styles parses correctly.
3. A blank in the middle of a row does not shift later cells (`r   r`).
4. Blank cells before the closing `|` are kept, and the row keeps its width.
5. A row with the wrong width gives an error that names the line.
6. A row label that is not the expected number gives an error that names the line.
7. Rulers are ignored. A grid with a bottom ruler, or with none, parses the same.
8. A character that is not in the legend gives an error that names the line and the character.

More tests:

9. Rows run from the top (`H-1`) to the bottom (0). The bottom-left character is cell (0,0).
10. The row count must equal the height in `size`. The column count must equal the width.
11. A character at an odd offset that is not a space is an error.
12. A row with no closing `|`, or no `|` at all, is an error.
13. A legend that uses `|`, `.` or a space as a key is an error.
14. `player_start` comes back as an object with that kind, and the cell is correct.
15. Comments and blank lines after the front matter are ignored. Windows line endings work.
16. Front matter errors: missing key, bad `size`, `origin` not `bottom-left`, legend kind not known to the game. Each names its line.
17. Objects come back in a fixed order (top row first, left to right), so tests and saved output are stable.

Game tests:

18. Long Way: the bundled `rocks-test.txt` parses. It has the grove at (12,2), (14,2), (12,4), (14,4), the green tree at (12,8) and the red tree at (18,8).
19. R-Town: the bundled `test.txt` parses. It has 68 big trees, 22 dead trees and one `player_start` at (40,30).

## Steps

1. **Spec first.** Rewrite `dev-docs/extensions/map-files.md` for the text format. No code yet.
2. **New parser beside the old one.** Add the text parser in `game-kit` as a second function, so both formats work during the move. Add `size` to `Map`. Write tests 1 to 17 first and see them fail, then write the code. Run `./dev-scripts/rust.sh test game-kit`.
3. **Compare old and new maps.** A temporary test parses `rocks-test.md` and `rocks-test.txt`, and `test.md` and `test.txt`, and checks the object sets are the same (the `.txt` files have the grove and the `@` as known extras). Delete this test in step 6.
4. **Switch Long Way.** Load `rocks-test.txt`. Update its kind list and its map tests (test 18). Run the game with `./dev-scripts/rust.sh script long-way "screenshot:grove;quit"` and look at the grove on screen. Say in the report what the screenshot did not show.
5. **Switch R-Town.** Load `test.txt`. Remove `MAP_CELLS` and use `Map.size`. Spawn the hero at the `player_start` cell. Add test 19. Run `script r-town` with a screenshot.
6. **Remove the old format.** Delete the markdown parser code (`Features`, `Placement`, `parse_body`, `parse_placement`, `parse_positions`, `parse_cells`, `pitch`), the temporary test, `rocks-test.md` and `test.md`.
7. **Check.** Run `./dev-scripts/rust.sh lint`, `test`, and `web` (a web compile check, because the map file is read with `include_str!`).
8. **Docs and records.**
   - Add `adr-x18` in `dev-docs/adr/EXTENSIONS.md` for the text format. Mark `adr-x11` as superseded. Update the adr-lw8 text in `games/long-way/docs/adr.md` and any R-Town ADR that names the map format.
   - Update `games/long-way/docs/maps.md`, `games/long-way/docs/source-layout.md`, `games/r-town/docs/README.md`, `SPEC.md` and `art-tags.md` for the new file names and format.
   - Update `dev-docs/ROADMAP.md` (items 3 and 4 name the old format), `dev-docs/extensions/README.md` and `dev-docs/bevy/notes.md` where they mention the markdown maps.
   - Link `dev-docs/RESEARCH-map-layouts.md` from the ADR and keep it as the research record.
9. **Draft a roadmap item for a visual map editor (to build later).** Add one item at the end of **Items for Future Implementation** in `dev-docs/ROADMAP.md`, numbered in sequence. Use an H3 heading and paragraphs for goal, considerations and known issues, as the roadmap rules require. Content to cover:
   - Goal: edit a map by clicking cells, and save it in the text format. The text format stays the source of truth, so hand editing keeps working.
   - Considerations: the editor must read and write the same files, and keep comments, rulers and blank footprint cells (a round trip must not change a file the user did not edit). Options are an in-game editor on top of the F2 grid and debug tools (Bevy, native and web), or a separate tool. The language rule applies (Rust, Lua, shell only, no Python), and any other language needs approval. Research notes on existing editors are in `dev-docs/RESEARCH-map-layouts.md`.
   - Known issues: no footprint data in the file, so the editor needs the game's sizes to draw footprints. Layers and multi-cell objects were left out of the format on purpose, and an editor may bring them back.
   - Ask the user to confirm the item text before the commit.
10. **Commit (ask first).** One commit for steps 1 to 9. Do not push.
11. **Migrate lasting content, then delete this plan.** User guides and troubleshooting go to `docs/` and `games/<game>/docs/`. Technical decisions go to `dev-docs/`. Delete `PLAN-text-maps.md`. Git history keeps it.

## Risks

- Footprint drawings are not checked against real sizes (the blank cells are visual only). The docs say so.
- Wide rows: R-Town rows are 159 characters plus the label. Some editors wrap them. Turn off word wrap when you edit.
- The `.txt` extension has no syntax colouring. Fixed-width reading depends on a fixed-width font.
- Both games change in one step. If one game breaks, the old parser stays available until step 6.
