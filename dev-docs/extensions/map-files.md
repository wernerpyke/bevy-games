# Map files

A text file that a human edits to draw a whole map. One character is one cell. The game reads the file and builds objects from the characters.

## Summary

- One grid shows every cell of the map and what is on it.
- A legend in the front matter says what each character means.
- Rulers and row labels help a human find a cell. Row labels are checked.
- Collision and sprites are built from the placed objects. No outline is drawn by hand.
- The parser is plain Rust behind one function, `parse`, so the format can change without touching the code that spawns objects.

## Status

| Part | Code | Shared? |
|---|---|---|
| Parser: `parse(file, source, kinds)`, `Map`, `MapObject`, `MapError` | `crates/game-kit/src/map.rs` | Shared. Object kinds are names (adr-x13). |
| Loading and `Kind` | `games/<game>/src/map.rs` | Per game |
| Spawning | per game | Per game |

Decisions: `adr-x18`, `adr-lw8`, `adr-rt7`. `adr-x11` is the markdown format that this one replaced. Research and options: `dev-docs/RESEARCH-map-layouts.md`.

## Specification

### Example

```
---
map: demo
cell: 16
origin: bottom-left
size: 6x4
legend:
  ".": empty
  "@": player_start
  "g": tree_big
---
   |0 1 2 3 4 5|
 3 |. . . . . .|
 2 |.     . . .|
 1 |. g   . @ .|
 0 |. . . . . .|
```

The tree is at (1,1). Its 2x2 footprint is drawn with blanks. The hero starts at (4,1).

The same map with `origin: top-left` has the same drawing and different row labels:

```
   |0 1 2 3 4 5|
 0 |. . . . . .|
 1 |.     . . .|
 2 |. g   . @ .|
 3 |. . . . . .|
```

The tree is at (1,2) in this file, and the game still sees it at (1,1). See rule 6.

### File layout

1. **Front matter** between two `---` lines, one `key: value` per line. A `#` starts a comment at the start of a line or after a space.
2. **Grid** after the second `---`. Each line is one of:
   - a **comment**: starts with `#`. Ignored.
   - a **blank line**. Ignored.
   - a **ruler line**: the text before the first `|` is blank. Ignored. Rulers may be at the top, at the bottom, both, or absent.
   - a **grid row**: `<label>|<cells>|`.

### Front matter keys

| Key | Needed | Meaning |
|---|---|---|
| `map` | yes | Map name, used in log and error messages |
| `cell` | yes | Grid cell size in pixels |
| `origin` | yes | `bottom-left` or `top-left`: the corner where row labels start at 0. See rule 6 |
| `size` | yes | Width and height in cells, written `WxH` |
| `legend` | yes | Maps each character in the grid to an object kind |
| `draw` | no | Drawn size of an object in pixels. The game gives the default |
| `jitter` | no | Largest random shift of each object in pixels. Default 0 |

3. A legend character is one character, in quotes. `.` always means empty and is written `".": empty`. The keys `.`, `|` and a space are reserved and cannot name an object kind.
4. The game defines the set of kinds. It passes the names it knows to `parse`, which reports any other name with its line. Objects carry the name as a `String`, and the game converts it to its own enum (adr-x13).
5. A legend entry can be a marker, such as `"@": player_start`. A marker is an ordinary object with that kind. The game reads its cell and does not spawn art for it.

### Grid rows

6. The label is the row number, right-aligned and zero-based. The parser checks every label. The first row on the page is the top row. Which number it has depends on `origin`:
   - `bottom-left`: the top row is `H-1` and the last row is 0, so labels count down the page. Labels match the F3 grid in a game that numbers cells from the bottom left.
   - `top-left`: the top row is 0 and the last row is `H-1`, so labels count down the page from 0, like rows of text.

   `origin` only changes how a file is written. `parse` always returns cells numbered from the bottom left with y up (`MapObject.cell`), so a game does not change when a file changes its origin. `Map.origin` says which one the file used, for tools that label cells.
7. The cells are the text between the first and the last `|`. Cell k, counting from 0 at the left, is the character at offset 2k. Characters at odd offsets must be spaces. The cell text is exactly `2W-1` characters long.
8. A blank or `.` is an empty cell. Any other character must be in the legend. A cell with an object is the anchor of that object.
9. The grid has exactly `H` rows, and each row has exactly `W` cells. Both numbers come from `size`.
10. In the game, cell (0,0) is the bottom-left cell. It is a fixed world position set by the game, so cells do not move when the window is resized.

### Footprints

11. An object occupies its anchor cell in the file. The size of its footprint belongs to the game, not to the file. The anchor is the cell at the bottom left of the footprint as drawn on the page, so a 2x2 footprint stretches one cell right and one cell up the page. This holds for both origins.
12. A person may draw the footprint cells with `.` or with blanks. The parser treats both as empty, so both give the same result. Blanks help a reader see how far an object reaches.
13. The parser does not check that a drawing matches the real footprint, and it does not report objects that overlap. Objects that are close enough merge into one collision cluster, and that is intended.

### Errors

14. Any format error stops loading and names the file and line, for example `assets/maps/rocks-test.txt:15: character x is not in the legend`.
15. Errors are:
    - front matter: a missing key, a bad `size`, an `origin` that is not `bottom-left` or `top-left`, a legend kind the game does not know, or a reserved legend key
    - a row with no `|`, or no closing `|`
    - a wrong row label
    - a row of the wrong width, or a non-space character at an odd offset
    - a character that is not in the legend
    - a row count that does not match the height

## Usage

- Add a new object kind to the game's name list and `Kind` enum, to the module that spawns it, and to the legend.
- Items that touch form a solid group in R-Town and Long Way, and an item that stands alone can be walked behind (`groups.md`).
- Keep rows in a fixed-width font and turn off word wrap. Large maps have long rows.
- Write blank cells as spaces inside the bars. The closing `|` keeps blanks at the end of a row safe from editors that remove trailing spaces.
- Long Way reads the map with `include_str!`, so a change needs a rebuild. R-Town reads the file from disk and reloads it on F1 (`adr-rt8`).

## Limits

- One grid, one object per cell. There are no layers and no ground tiles in the file.
- One map per game build.
- Maps with triggers, paths or nested data may outgrow this format (roadmap items 3 and 4).
- A visual editor is a roadmap item. It must keep this format as the source of truth.

## Tests

Inline in `game-kit` `map.rs`. Each game tests that its own map file parses (counts of objects, size and the start marker). The parser tests cover:

- Footprints: a dotted and a blank footprint give the same objects, one grid may mix both styles, a blank in the middle of a row does not shift later cells, and blanks before the closing `|` are kept.
- Grid shape: rows run top to bottom and cells are numbered from the bottom left, the row count and the width must match `size`, a character at an odd offset must be a space, and a row needs both bars.
- Labels and rulers: wrong row labels name their line (for both origins), and rulers are ignored at the top, at the bottom, at both ends or absent.
- Origin: `top-left` and `bottom-left` files with the same drawing give the same objects, and an unknown origin is an error.
- Legend: a character that is not in the legend names its line, reserved keys (`|`, `.`, space) are errors, and a marker such as `player_start` comes back as an ordinary object.
- Front matter: missing keys, a bad `size`, unknown keys (including the removed `pitch`), bad `draw`, `jitter` and `cell` values, an unknown kind, comments and optional keys.
- Other: comments, blank lines and Windows line endings, and objects come back in a fixed order (top row first, then left to right).
