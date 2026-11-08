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
| Exits and entries: `Area`, `MapExit`, `MapEntry`, the in-file checks | `crates/game-kit/src/map_areas.rs` | Shared |
| Background: `Background`, `BackgroundMode`, and `place_tiles` (which cells get which image) | `crates/game-kit/src/map.rs`, `crates/game-kit/src/map_background.rs` | Shared. The game draws the images |
| Links between maps: `validate_links` | `crates/game-kit/src/map_links.rs` | Shared. Run by the map registry. |
| Loading and `Kind` | `games/<game>/src/map.rs` | Per game |
| Spawning | per game | Per game |

Decisions: `adr-x18`, `adr-x30` (exits and entries), `adr-x33` (exit objects), `adr-x36` (backgrounds), `adr-lw8`, `adr-rt7`. `adr-x11` is the markdown format that this one replaced. Research and options: `dev-docs/RESEARCH-map-layouts.md`.

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

The tree is at (1,1). Its 2x2 footprint is drawn with blanks. The player starts at (4,1).

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
| `exits` | no | Zones that lead to other maps. See "Exits and entries" |
| `entries` | no | Places where the player arrives from another map. See "Exits and entries" |
| `background` | no | The ground of the map: a colour and tile images. See rule 25 |

3. A legend character is one character, in quotes. `.` always means empty and is written `".": empty`. The keys `.`, `|` and a space are reserved and cannot name an object kind.
4. The game defines the set of kinds. It passes the names it knows to `parse`, which reports any other name with its line. Objects carry the name as a `String`, and the game converts it to its own enum (adr-x13).
5. A legend entry can be a marker, such as `"@": player_start`. A marker is an ordinary object with that kind. The game reads its cell and does not spawn art for it.
   A legend entry may have parameters after the kind: `"S": spawn_skeleton every=10 strategy=patrol radius=120`. Each parameter is `key=value` with no spaces, the key has letters, digits and `_`, and a key is not given twice. The parser reads them into `MapObject.params` (`Params`: `get`, `f32`, `line`) and does not know what they mean. The game checks the keys it needs and reports problems. An entry without parameters is unchanged.

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

14. Any format error stops loading and names the file and line, for example `assets/maps/1.txt:15: character x is not in the legend`.
15. Errors are:
    - exits and entries: see rules 16 to 22, each error names the line of the item
    - front matter: a missing key, a bad `size`, an `origin` that is not `bottom-left` or `top-left`, a legend kind the game does not know, or a reserved legend key, or a bad or repeated legend parameter
    - a row with no `|`, or no closing `|`
    - a wrong row label
    - a row of the wrong width, or a non-space character at an odd offset
    - a character that is not in the legend
    - a row count that does not match the height

### Exits and entries

Two optional blocks in the front matter describe how maps join. The game reads them when it moves the player between maps (`dev-docs/extensions/map-transitions.md`). The parser reads and checks them now.

```
exits:
  to_2: area=3..6,9 to=2 entry=from_1 transition=scene
entries:
  from_1: area=3..6,0 facing=up
```

16. One line is `name: key=value ...`, indented under `exits:` or `entries:`. Names are unique inside one block. An entry and an exit may share a name.
17. `area` is a straight strip of cells: `x0..x1,y`, `x,y0..y1` or one cell `x,y`. A range on both axes is an error, so an area is never a block or a diagonal. A range goes from low to high. Cells are numbered like the row labels of the file, so a `top-left` file counts rows from the top. The area must be inside the map.
18. An **exit** needs `area`, `to` (the id of the target map, set by the game), `entry` (the name of an entry in that map) and `transition` (`seamless` or `scene`). Its area lies along a map edge: a horizontal strip on the top or bottom row, a vertical strip in the left or right column. A single cell in a corner is an error, because it touches two edges.
19. An **entry** needs `area` and may have `facing` (`up`, `down`, `left` or `right`).
20. Mark the cells of an area with a blank (`' '`) in the grid, not `.`. This is a reading aid. The parser treats both the same (rule 12) and does not check it.
21. No object may stand in an area. The one exception is a `player_start` marker in an entry area. Two exits may not overlap, and an entry may not overlap an exit (the player would be sent back at once).
22. Between maps (`validate_links`, when a game registers its maps): the exit leads to a registered map and an entry of that map, and the two have the same length, so the position along the strip carries over. A seamless exit needs its entry on the opposite edge (a top exit joins a bottom entry) and may not lead to its own map.
23. **Start marker.** `"@": player_start` is an ordinary marker (rule 5). A game reads its cell as the start of the player. A map that is only entered from another map needs none. Add one to a later map to be able to start in it directly, for example for a scripted run.
   `start_feet(map, origin)` gives its position (the centre of the cell).
24. **Exit objects.** A legend entry may have `exit_to=<map id>`, for example `"m": marker exit_to=2` (`adr-x33`). Touching the object sends the player to that map, at its `player_start`. The parser reads it as an ordinary parameter (`MapObject::exit_to`). Between maps (`validate_links`) the map must be registered and have exactly one `player_start`, and the error names the legend line. The game decides where the object can be touched (`ExitTrigger`). There is no `transition` key: it is always a scene transition. Do not put such an object inside an exit area (rule 21). A map may use exit areas, exit objects, or both.

### Background

25. **Background.** An optional `background:` block in the front matter gives the ground of the map: a colour, and tile images drawn over it (`adr-x36`). Indented `key: value` lines, like the legend:

    ```
    background:
      color: "#292829"
      mode: random
      fill: 0.2
      jitter: 16
      scale: 2
      tiles: grass_0 grass_1 flowers_0
    ```

    | Key | Needed | Meaning |
    |---|---|---|
    | `color` | yes | Hex colour `#rrggbb`. Quote it: an unquoted `#` starts a comment |
    | `mode` | yes | `random` or `pattern`. No default. `pattern` (a repeating fill) is reserved: the parser rejects it with "not implemented yet" |
    | `fill` | no | Chance, 0 to 1, that a cell gets an image. Default 0 (colour only) |
    | `tiles` | with `fill` above 0 | Image names separated by blanks, each equally likely. The game checks the names, the kit does not know the art |
    | `jitter` | no | Largest random shift of an image in pixels on each axis. Default 0. Separate from the map key `jitter` |
    | `scale` | no | A number above 0, the size of an image as a multiple of its art size. Default 1. `0.5` is allowed |
    | `seed` | no | A whole number that changes the dice. Default 0 |

    The parser (`map.rs`, `Map::background`) checks the syntax and names the line of each error: a missing `color` or `mode`, a bad `mode`, colour, `fill`, `jitter`, `scale` or `seed`, a `fill` above 0 without `tiles`, a repeated tile name, an unknown key, a key or the block given twice.
    `map_background::place_tiles` decides which cells get an image, which image, and where. Each cell rolls its own dice from a hash of the map name, the `seed` and the cell number. So the ground is the same on every load, on F1 reload and on the web, it does not depend on the order of the cells, and a map that grows keeps the images of its old cells. The number of images varies from map to map, because it is a chance per cell. An image is centred on its cell, shifted by the jitter, and moved inwards so it stays inside the map. The game draws the colour as a rectangle of the map size and the images above it (`layers.rs`, `BACKGROUND_Z` and `BACKGROUND_TILES_Z`).

## Usage

- Add a new object kind to the game's name list and `Kind` enum, to the module that spawns it, and to the legend.
- Items that touch form a solid group in R-Town and Long Way, and an item that stands alone can be walked behind (`groups.md`).
- Keep rows in a fixed-width font and turn off word wrap. Large maps have long rows.
- Write blank cells as spaces inside the bars. The closing `|` keeps blanks at the end of a row safe from editors that remove trailing spaces.
- Long Way reads the map with `include_str!`, so a change needs a rebuild. R-Town reads the file from disk and reloads it on F1 (`adr-rt8`).

## Limits

- One grid, one object per cell. There are no layers and no ground tiles in the file.
- One map file is one map. How a game moves between maps is in `dev-docs/extensions/map-transitions.md`.
- Maps with triggers, paths or nested data may outgrow this format (roadmap items 4 and 5).
- A visual editor is a roadmap item. It must keep this format as the source of truth.

## Tests

Inline in `game-kit` `map.rs`. Each game tests that its own map file parses (counts of objects, size and the start marker). The parser tests cover:

- Footprints: a dotted and a blank footprint give the same objects, one grid may mix both styles, a blank in the middle of a row does not shift later cells, and blanks before the closing `|` are kept.
- Grid shape: rows run top to bottom and cells are numbered from the bottom left, the row count and the width must match `size`, a character at an odd offset must be a space, and a row needs both bars.
- Labels and rulers: wrong row labels name their line (for both origins), and rulers are ignored at the top, at the bottom, at both ends or absent.
- Origin: `top-left` and `bottom-left` files with the same drawing give the same objects, and an unknown origin is an error.
- Legend: a character that is not in the legend names its line, reserved keys (`|`, `.`, space) are errors, and a marker such as `player_start` comes back as an ordinary object.
- Front matter: missing keys, a bad `size`, unknown keys (including the removed `pitch`), bad `draw`, `jitter` and `cell` values, an unknown kind, comments and optional keys.
- Exits and entries: `map_areas.rs` tests the area notation, the edge rules, objects in areas, overlaps and a `top-left` file. `map_links.rs` tests the checks between maps. It also tests `exit_to` (unknown map, a target without exactly one start). `map.rs` tests `exit_to` and `start_feet`.
- Background: a full block, defaults, a block with only a colour and a mode, no block, every error with its line, `scale` (including `0.5`), a repeated key or block, and the map key `jitter` kept apart. `map_background.rs` tests the placement: fill 0 and 1, the same result for the same input, a different seed, the share close to `fill`, jitter and the clamp inside the map, a map that grows, and a fixed expected list so the hash never changes.
- Other: comments, blank lines and Windows line endings, and objects come back in a fixed order (top row first, then left to right).
